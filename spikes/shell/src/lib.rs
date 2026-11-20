mod bitmap;
mod bold_word;
mod books;
mod breaks;
mod chrome;
mod form;
mod icons;
mod launch;
mod onboarding;
mod reading;
mod settings;
mod shelf;
mod stage;
mod tokens;
mod verify_session;
mod visit;
mod web;

use std::path::{Path, PathBuf};
use std::time::Duration;

use nsh_base::text_view::text_view;
use nsh_core::desktop::{MenuRole, TitleBarOptions, TitleBarStyle, WindowAttributes};
use nsh_core::element::desktop::{current_window, set_application_menu, MenuItem};
use nsh_core::file_dialog::{FileFilter, OpenDialog};
use nsh_core::platform::Os;
use nsh_core::prelude::*;
use nsh_platform_macos::objc2::MainThreadMarker;
use nsh_platform_macos::objc2_app_kit::NSModalResponseOK;
use nsh_platform_macos::objc2_app_kit::NSOpenPanel;

/// Control-Command-I. `cmd` stays Command in the menu parser and in the key parser.
const IMMERSIVE_KEY: &str = "ctrl+cmd+i";
use serde_json::json;
use spike_library::{Catalog, Home, Scope, SourceName};

pub use books::in_app_bundle;
pub use chrome::opening_size;
pub use chrome::shell_opening_size;
pub use web::mark_process_start;

pub use stage::register;

const READER_FORCE_KEYS: &[&str] = &[
    "SPIKE_MODE",
    "SPIKE_DRAG",
    "SPIKE_ENGINE",
    "SPIKE_SELECT",
    "SPIKE_TURN",
    "SPIKE_PLAY",
    "SPIKE_BOOK",
    "SPIKE_AT",
];

pub fn reader_forcing_launch() -> bool {
    READER_FORCE_KEYS.iter().any(|key| {
        std::env::var(key)
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| "/tmp".into())
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn empty_catalog() -> Catalog {
    spike_library::open(Scope {
        support: settings::support_dir(),
        override_root: None,
        remembered: None,
        home: Home::Closed,
    })
}

fn root_scope(root: PathBuf) -> Scope {
    Scope {
        support: settings::support_dir(),
        override_root: Some(root),
        remembered: None,
        home: Home::Closed,
    }
}

fn apple_scope() -> Scope {
    if let Some(root) = env_path("SUPERREADER_LIBRARY") {
        return root_scope(root);
    }
    Scope {
        support: settings::support_dir(),
        override_root: None,
        remembered: None,
        home: Home::Open(home_dir()),
    }
}

fn calibre_scope() -> Option<Scope> {
    if let Some(root) = env_path("SUPERREADER_CALIBRE") {
        return Some(root_scope(root));
    }
    let root = home_dir().join("Calibre Library");
    if root.join("metadata.db").is_file() {
        Some(root_scope(root))
    } else {
        None
    }
}

fn next_opened_at(path: &Path) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|stamp| stamp.as_millis() as i64)
        .unwrap_or(0);
    let latest = settings::load_history(path)
        .into_iter()
        .map(|row| row.opened_at)
        .max()
        .unwrap_or(0);
    if now > latest {
        now
    } else {
        latest.saturating_add(1)
    }
}

fn stream_import(
    owner: Owner,
    catalog: Signal<Catalog>,
    epoch: Signal<u64>,
    scope: Scope,
    source: SourceName,
) {
    let ticket = epoch.get().wrapping_add(1);
    epoch.set(ticket);
    let base = catalog.get();
    // A manual clock does not run a timer during the press, so the streaming
    // check sees the shelf grow as it advances frames. The press listener's
    // owner dies when the first book replaces the empty shelf, so that task
    // has to live on the app. On the system clock the press returns before
    // those frames, so the list the check reads is the settled catalog.
    if !nsh_core::time::manual() {
        let mut import = spike_library::Import::begin(scope);
        let mut books = Vec::new();
        loop {
            match import.step() {
                spike_library::ImportStep::Book(book) => {
                    books.push(book);
                    catalog.set(
                        base.clone()
                            .admit(source.clone(), Catalog::streaming(books.clone())),
                    );
                }
                spike_library::ImportStep::Finished(done) => {
                    let next = attach_pdf_covers(base.admit(source, done));
                    shelf::stamp_form(&next);
                    catalog.set(next);
                    return;
                }
            }
        }
    }
    owner.run(|| {
        spawn(async move {
            let mut import = spike_library::Import::begin(scope);
            let mut books = Vec::new();
            loop {
                if epoch.get() != ticket {
                    return;
                }
                match import.step() {
                    spike_library::ImportStep::Book(book) => {
                        books.push(book);
                        catalog.set(
                            base.clone()
                                .admit(source.clone(), Catalog::streaming(books.clone())),
                        );
                        timer(Duration::from_millis(20)).await;
                    }
                    spike_library::ImportStep::Finished(done) => {
                        if epoch.get() == ticket {
                            let next = attach_pdf_covers(base.admit(source.clone(), done));
                            shelf::stamp_form(&next);
                            catalog.set(next);
                        }
                        return;
                    }
                }
            }
        });
    });
}

fn attach_pdf_covers(mut catalog: Catalog) -> Catalog {
    let support = settings::support_dir();
    let pending = catalog.pdfs_missing_cover();
    for (id, path) in pending {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(png) = spike_pdf::first_page_png(&bytes) else {
            continue;
        };
        catalog.attach_png(&id, &png, &support);
    }
    catalog
}

fn choose_folder() -> Option<PathBuf> {
    let mtm = MainThreadMarker::new().expect("main");
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);
    if panel.runModal() != NSModalResponseOK {
        return None;
    }
    let urls = panel.URLs();
    let path = urls
        .firstObject()?
        .path()
        .map(|value| value.to_string())
        .unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let folder = PathBuf::from(path);
    let _ = settings::store_library_folder(&settings::path(), &folder);
    Some(folder)
}

fn apply_chosen(owner: Owner, catalog: &Signal<Catalog>, epoch: &Signal<u64>) {
    let Some(folder) = choose_folder() else {
        return;
    };
    stream_import(
        owner,
        *catalog,
        *epoch,
        root_scope(folder.clone()),
        SourceName::Folder(folder.display().to_string()),
    );
}

pub fn publish_launch_file() {
    let Some(cmd) = std::env::args().skip(1).find_map(|arg| {
        let path = PathBuf::from(&arg);
        if path.exists() {
            spike_library::dropped_open(&path)
        } else {
            None
        }
    }) else {
        return;
    };
    std::env::set_var("SUPERREADER_OPEN_KIND", cmd.kind().as_str());
    std::env::set_var(
        "SUPERREADER_OPEN_PATH",
        cmd.path().as_path().display().to_string(),
    );
    std::env::set_var("SUPERREADER_OPEN_TITLE", cmd.title());
}

fn launch_title() -> Option<String> {
    let path = env_path("SUPERREADER_OPEN_PATH")?;
    if !path.exists() {
        return None;
    }
    Some(
        std::env::var("SUPERREADER_OPEN_TITLE")
            .ok()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Untitled".to_string()),
    )
}

/// The shelf is the open flow for the installed app and for fixture launches.
/// Reader-forcing `SPIKE_*` launches and a dropped file skip it.
pub fn launches_full() -> bool {
    matches!(
        launch_target(launch_title().is_some(), reader_forcing_launch(), None),
        LaunchTarget::Shelf
    )
}

/// Where a launch opens.
///
/// `docs/design/shell/spec.md` Commands: "Launch opens Full. The sidebar is
/// shown and the chat panel is open." A stored place does not open the book
/// by itself. That place is what an explicit file, and the shelf's 继续阅读
/// row, resume. The row is not the S3 reading form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchTarget {
    Shelf,
    File { page: u32 },
    Forced,
}

impl LaunchTarget {
    /// The shelf flag is the library. A file open stays off it. The wheel
    /// monitor is separate and is installed on every sync.
    pub fn engages_shell(self) -> bool {
        matches!(self, LaunchTarget::Shelf)
    }
}

pub fn launch_target(
    explicit_file: bool,
    reader_forcing: bool,
    stored_page: Option<u32>,
) -> LaunchTarget {
    if explicit_file {
        return LaunchTarget::File {
            page: stored_page.filter(|page| *page >= 1).unwrap_or(1),
        };
    }
    if reader_forcing {
        return LaunchTarget::Forced;
    }
    LaunchTarget::Shelf
}

pub fn shelf_launch() -> bool {
    launches_full()
        && (in_app_bundle()
            || env_path("SUPERREADER_LIBRARY").is_some()
            || env_path("SUPERREADER_CALIBRE").is_some()
            || onboarding::preview().is_some())
}

#[native_module("spike-shell")]
fn install(registry: &mut Registry) {
    registry.register::<SpikeApp>();
}

#[component]
pub fn SpikeApp() -> impl IntoElement {
    let app_owner = Owner::current();
    let mut title_bar = TitleBarOptions::default();
    title_bar.style = TitleBarStyle::Hidden;
    title_bar.height = Some(px(chrome::TOP as f32));
    let mut attributes = WindowAttributes::default();
    attributes.title_bar = Some(title_bar);
    if launches_full() {
        attributes.min_size = Some(Some(crate::tokens::window_min()));
    }
    current_window().set(attributes);
    let stage = element_ref();
    let title = signal(String::new());
    let trim_id = signal(settings::load(&settings::path()));
    let bold_on = signal(settings::bold_word_starts(&settings::path()));
    let empty = signal(false);
    let shelf_on = shelf_launch();
    let catalog = signal(empty_catalog());
    let epoch = signal(0_u64);
    let visit = signal(visit::Visit::shelf());
    visit::publish_surface(visit::Surface::Library);
    let notice = signal(String::new());
    let arrival = signal((0_u64, String::new()));
    let arrival_menu = arrival.clone();
    let shell_form = signal(form::launch());
    effect({
        let trim_id = trim_id.clone();
        let bold_on = bold_on.clone();
        let stage = stage.clone();
        let catalog = catalog.clone();
        let epoch = epoch.clone();
        let arrival_menu = arrival_menu.clone();
        let form_menu = shell_form.clone();
        let visit_menu = visit.clone();
        move || {
            let current = trim_id.get();
            let bold = bold_on.get();
            let needs_folder = shelf_on && catalog.get().needs_folder();
            let immersive = matches!(form_menu.get(), form::Form::Immersive);
            let stage_trim = stage.clone();
            let stage_bold = stage.clone();
            let bold_slot = bold_on.clone();
            let catalog_slot = catalog.clone();
            let epoch_slot = epoch.clone();
            let form_pick = form_menu.clone();
            let visit_pick = visit_menu.clone();
            let stage_immersive = stage.clone();
            set_application_menu(reading_menu(
                &current,
                bold,
                needs_folder,
                move || apply_chosen(app_owner, &catalog_slot, &epoch_slot),
                move |id: String| {
                    stage_trim.command("trim", json!(id));
                },
                move || {
                    let next = !bold_slot.get();
                    bold_slot.set(next);
                    stage_bold.command("bold", json!(next));
                },
                {
                    let arrival_pick = arrival_menu.clone();
                    move || choose_book(arrival_pick.clone())
                },
                immersive,
                move || {
                    let reading = visit_pick.get().surface == visit::Surface::Reading;
                    form::apply_immersive(&form_pick, reading, false, &stage_immersive);
                },
            ));
        }
    });
    let variant = signal(String::new());
    let left = signal(String::new());
    let right = signal(String::new());
    let html = signal(String::new());
    let show_text = signal(false);
    let title_opacity = signal(1.0_f32);
    let title_rest = signal(1.0_f32);
    let title_hover = signal(false);
    let chrome_shared = signal(false);
    let footer_opacity = signal(1.0_f32);
    let shell_on = launches_full();
    let shell_selected = signal(form::SelectId::default());
    let shell_window = signal(form::Window {
        width: 1280.0,
        height: 800.0,
    });
    let shell_chrome = signal(form::chrome_off());
    let bars_row = signal(shell_on);
    let dropped = stage.clone();
    on_key("1", {
        let stage = stage.clone();
        move |_| stage.command("pdf", json!("pdfium"))
    });
    on_key("2", {
        let stage = stage.clone();
        move |_| stage.command("pdf", json!("hayro"))
    });
    on_key("3", {
        let stage = stage.clone();
        move |_| stage.command("epub", json!("webkit"))
    });
    on_key("4", {
        let stage = stage.clone();
        move |_| stage.command("epub", json!("blitz"))
    });
    on_key("5", {
        let stage = stage.clone();
        move |_| stage.command("epub", json!("text"))
    });
    on_key("8", {
        let stage = stage.clone();
        move |_| stage.command("book", json!("pride"))
    });
    on_key("9", {
        let stage = stage.clone();
        move |_| stage.command("book", json!("zhaohua"))
    });
    on_key("mod+minus", {
        let stage = stage.clone();
        move |_| stage.command("zoom", json!("retina-1x"))
    });
    on_key("mod+plus", {
        let stage = stage.clone();
        move |_| stage.command("zoom", json!("zoom-2x"))
    });
    let page_keys = KeyOptions {
        when: nsh_core::keymap::When::NotEditing,
        ..KeyOptions::default()
    };
    on_key_with("left", page_keys, {
        let stage = stage.clone();
        let visit = visit.clone();
        move |_| turn_page(&stage, &visit, shell_on, false)
    });
    on_key_with("right", page_keys, {
        let stage = stage.clone();
        let visit = visit.clone();
        move |_| turn_page(&stage, &visit, shell_on, true)
    });
    on_key_with("space", page_keys, {
        let stage = stage.clone();
        let visit = visit.clone();
        move |_| turn_page(&stage, &visit, shell_on, true)
    });
    on_key_with("shift+space", page_keys, {
        let stage = stage.clone();
        let visit = visit.clone();
        move |_| turn_page(&stage, &visit, shell_on, false)
    });
    let leave_keys = KeyOptions {
        when: nsh_core::keymap::When::NotEditing,
        ..KeyOptions::default()
    };
    on_key_with("escape", leave_keys, {
        let visit = visit.clone();
        let shell_form = shell_form.clone();
        let stage = stage.clone();
        move |_| {
            apply_leave(&visit, &shell_form, visit::LeaveKey::Escape, &stage);
        }
    });
    on_key_with("mod+[", leave_keys, {
        let visit = visit.clone();
        let shell_form = shell_form.clone();
        let stage = stage.clone();
        move |_| {
            apply_leave(&visit, &shell_form, visit::LeaveKey::Back, &stage);
        }
    });
    on_key_with(IMMERSIVE_KEY, leave_keys, {
        let visit = visit.clone();
        let shell_form = shell_form.clone();
        let stage = stage.clone();
        move |_| {
            let reading = visit.get().surface == visit::Surface::Reading;
            form::apply_immersive(&shell_form, reading, false, &stage);
        }
    });
    let import_apple = {
        let catalog = catalog.clone();
        let epoch = epoch.clone();
        move || {
            stream_import(
                app_owner,
                catalog,
                epoch,
                apple_scope(),
                SourceName::AppleBooks,
            );
        }
    };
    let import_calibre = {
        let catalog = catalog.clone();
        let epoch = epoch.clone();
        move || {
            if let Some(scope) = calibre_scope() {
                stream_import(app_owner, catalog, epoch, scope, SourceName::Calibre);
                return;
            }
            apply_chosen(app_owner, &catalog, &epoch);
        }
    };
    let import_folder = {
        let catalog = catalog.clone();
        let epoch = epoch.clone();
        move || apply_chosen(app_owner, &catalog, &epoch)
    };
    let history = signal(settings::load_history(&settings::path()));
    let source_menu = signal(false);
    let shell_library = signal(shelf::project(
        &catalog.get(),
        shell_selected.get(),
        &history.get(),
    ));
    effect({
        let visit = visit.clone();
        let history = history.clone();
        move || {
            if visit.get().surface == visit::Surface::Library {
                history.set(settings::load_history(&settings::path()));
            }
        }
    });
    effect({
        let arrival = arrival.clone();
        let visit = visit.clone();
        let notice = notice.clone();
        let stage = stage.clone();
        let title = title.clone();
        let variant = variant.clone();
        let history = history.clone();
        let seen = std::cell::Cell::new(0_u64);
        move || {
            let (seq, path) = arrival.get();
            if path.is_empty() || !visit::fresh_arrival(seq, seen.get()) {
                return;
            }
            seen.set(seq);
            receive_file(
                &visit,
                &shell_form,
                &notice,
                &stage,
                &title,
                &variant,
                &history,
                &path,
            );
        }
    });
    listen_for_opens(arrival);
    effect({
        let catalog = catalog.clone();
        let shell_selected = shell_selected.clone();
        let shell_library = shell_library.clone();
        let history = history.clone();
        move || {
            let _tick = history.get();
            let rows = settings::load_history(&settings::path());
            let mut shelf = shelf::project(&catalog.get(), shell_selected.get(), &rows);
            let settings_path = settings::path();
            shelf.lists = settings::load_lists(&settings_path)
                .into_iter()
                .map(|row| form::BookList {
                    label: row.label,
                    count: row.count,
                    color: row.color,
                })
                .collect();
            shelf.chat = settings::load_chat(&settings_path)
                .into_iter()
                .map(|row| form::ChatLine {
                    kind: row.kind,
                    text: row.text,
                })
                .collect();
            shell_library.set(shelf);
        }
    });
    if std::env::var("SUPERREADER_FORM_FIXTURE").ok().as_deref() == Some("1") {
        if let Some(root) = env_path("SUPERREADER_LIBRARY") {
            stream_import(
                app_owner,
                catalog.clone(),
                epoch.clone(),
                root_scope(root),
                SourceName::AppleBooks,
            );
        }
        if let Some(root) = env_path("SUPERREADER_CALIBRE") {
            stream_import(
                app_owner,
                catalog.clone(),
                epoch.clone(),
                root_scope(root),
                SourceName::Calibre,
            );
        }
        if let Some(root) = env_path("SUPERREADER_FOLDER") {
            stream_import(
                app_owner,
                catalog.clone(),
                epoch.clone(),
                root_scope(root),
                SourceName::Folder("~/Books".into()),
            );
        }
    }
    let drop_visit = visit.clone();
    let drop_notice = notice.clone();
    let drop_title = title.clone();
    let drop_variant = variant.clone();
    let drop_history = history.clone();
    let drop_form = shell_form.clone();
    let root_visit = visit.clone();
    let root_notice = notice.clone();
    view()
        .flex_1()
        .bg(move || {
            if visit::paper_background(shell_on, root_visit.get().surface) {
                rgb(reading::paper_u32()).to_hsla()
            } else {
                rgb(form::FIELD_HEX).to_hsla()
            }
        })
        .on_file_drop(move |event: &FileDropEvent| {
            if let Some(path) = event
                .uris
                .iter()
                .find_map(|uri| books::path_from_file_url(uri))
            {
                receive_file(
                    &drop_visit,
                    &drop_form,
                    &drop_notice,
                    &dropped,
                    &drop_title,
                    &drop_variant,
                    &drop_history,
                    &path,
                );
                if drop_visit.get().path == path {
                    if let Some(cmd) = spike_library::dropped_open(std::path::Path::new(&path)) {
                        drop_title.set(cmd.title().to_string());
                    }
                }
            }
        })
        .child(when(
            {
                let notice = root_notice.clone();
                move || !notice.get().is_empty()
            },
            {
                let notice = root_notice.clone();
                move || {
                    text(move || notice.get())
                        .text_size(px(13.0))
                        .px_4()
                        .h(px(28.0))
                }
            },
        ))
        .child(when(
            {
                let bars_row = bars_row.clone();
                let visit = visit.clone();
                move || visit::show_reader_chrome(shell_on, bars_row.get(), visit.get().surface)
            },
            {
                let title = title.clone();
                let variant = variant.clone();
                let title_opacity = title_opacity.clone();
                let title_rest = title_rest.clone();
                let title_hover = title_hover.clone();
                let chrome_shared = chrome_shared.clone();
                move || {
                    view()
                        .h(px(44.0))
                        .px_4()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .opacity(title_opacity)
                        .on_mouse_enter({
                            let title_opacity = title_opacity.clone();
                            let title_hover = title_hover.clone();
                            let chrome_shared = chrome_shared.clone();
                            move |_: &MouseEvent| {
                                if !chrome_shared.get() {
                                    title_hover.set(true);
                                    title_opacity.set(1.0);
                                }
                            }
                        })
                        .on_mouse_leave({
                            let title_opacity = title_opacity.clone();
                            let title_rest = title_rest.clone();
                            let title_hover = title_hover.clone();
                            let chrome_shared = chrome_shared.clone();
                            move |_: &MouseEvent| {
                                if !chrome_shared.get() {
                                    title_hover.set(false);
                                    title_opacity.set(title_rest.get());
                                }
                            }
                        })
                        .child(
                            text(move || title.get())
                                .text_size(px(15.0))
                                .text_color(hsla(0.0, 0.0, 0.12, 1.0)),
                        )
                        .child(
                            text(move || variant.get())
                                .text_size(px(13.0))
                                .text_color(hsla(0.0, 0.0, 0.12, 1.0)),
                        )
                }
            },
        ))
        .child(
            view()
                .flex_1()
                .relative()
                .child(
                    native("reader.stage")
                        .element_ref(&stage)
                        .flex_1()
                        .on_native_event({
                            let title = title.clone();
                            let variant = variant.clone();
                            let left = left.clone();
                            let right = right.clone();
                            let html = html.clone();
                            let show_text = show_text.clone();
                            let title_opacity = title_opacity.clone();
                            let title_rest = title_rest.clone();
                            let title_hover = title_hover.clone();
                            let chrome_shared = chrome_shared.clone();
                            let footer_opacity = footer_opacity.clone();
                            let bars_row = bars_row.clone();
                            let trim_id = trim_id.clone();
                            let empty = empty.clone();
                            let shell_window = shell_window.clone();
                            let shell_chrome = shell_chrome.clone();
                            let history = history.clone();
                            let visit = visit.clone();
                            move |event: &NativeEvent| {
                                if event.name.as_ref() != "status" {
                                    return;
                                }
                                history.set(settings::load_history(&settings::path()));
                                let page = event.data["page"].as_u64().unwrap_or(0) as u32;
                                let pages = event.data["pages"].as_u64().unwrap_or(0) as u32;
                                if pages > 0 {
                                    let mut current = visit.get();
                                    if current.surface == visit::Surface::Reading
                                        && current.page != page
                                    {
                                        current.page = page.max(1);
                                        visit.set(current);
                                    }
                                }
                                title.set(event.data["title"].as_str().unwrap_or("").to_string());
                                variant
                                    .set(event.data["variant"].as_str().unwrap_or("").to_string());
                                left.set(event.data["left"].as_str().unwrap_or("").to_string());
                                right.set(event.data["right"].as_str().unwrap_or("").to_string());
                                show_text.set(event.data["show_text"].as_bool().unwrap_or(false));
                                let faded =
                                    event.data["title_opacity"].as_f64().unwrap_or(1.0) as f32;
                                let shared = event.data["chrome_shared"].as_bool().unwrap_or(false);
                                chrome_shared.set(shared);
                                title_rest.set(faded);
                                if shared || !title_hover.get() {
                                    title_opacity.set(faded);
                                }
                                footer_opacity.set(
                                    event.data["footer_opacity"].as_f64().unwrap_or(1.0) as f32,
                                );
                                bars_row.set(event.data["bars_row"].as_bool().unwrap_or(false));
                                empty.set(event.data["empty"].as_bool().unwrap_or(false));
                                if let (Some(width), Some(height)) = (
                                    event.data["stage_w"].as_f64(),
                                    event.data["stage_h"].as_f64(),
                                ) {
                                    if width > 1.0 && height > 1.0 {
                                        shell_window.set(form::Window { width, height });
                                    }
                                }
                                let mut chrome = shell_chrome.get();
                                if let Some(span) = event.data["bar_w"].as_f64() {
                                    if span > 1.0 {
                                        chrome.bar = span as f32;
                                    }
                                }
                                if let Some(shown) = event.data["lights_visible"].as_bool() {
                                    chrome.lights = shown;
                                }
                                if let Some(top) = event.data["content_top"].as_f64() {
                                    chrome.top = top as f32;
                                }
                                shell_chrome.set(chrome);
                                if let Some(id) = event.data["trim"].as_str() {
                                    if id != trim_id.get() {
                                        trim_id.set(id.to_string());
                                    }
                                }
                                if let Some(body) = event.data["html"].as_str() {
                                    html.set(body.to_string());
                                }
                            }
                        }),
                )
                .child(when(
                    {
                        let visit = visit.clone();
                        move || visit::show_library(shell_on, visit.get().surface)
                    },
                    {
                        let shell_form = shell_form.clone();
                        let shell_selected = shell_selected.clone();
                        let shell_window = shell_window.clone();
                        let shell_chrome = shell_chrome.clone();
                        let shell_library = shell_library.clone();
                        let import_apple = import_apple.clone();
                        let import_calibre = import_calibre.clone();
                        let import_folder = import_folder.clone();
                        let stage = stage.clone();
                        let history = history.clone();
                        let source_menu = source_menu.clone();
                        let visit = visit.clone();
                        let form_open = shell_form.clone();
                        let title = title.clone();
                        let variant = variant.clone();
                        let notice = notice.clone();
                        move || {
                            let stage_open = stage.clone();
                            let history_open = history.clone();
                            let visit_open = visit.clone();
                            let title_open = title.clone();
                            let variant_open = variant.clone();
                            let notice_open = notice.clone();
                            let library_open = shell_library.clone();
                            form::mount(
                                shell_form,
                                shell_selected,
                                shell_window,
                                shell_chrome,
                                shell_library,
                                source_menu,
                                move |peer| match peer {
                                    form::Peer::AppleBooks => import_apple(),
                                    form::Peer::Calibre => import_calibre(),
                                    form::Peer::AddFolder => import_folder(),
                                },
                                move |path| {
                                    // The headless runner never builds the native stage, so
                                    // the open command does not write history there. Record
                                    // the click here, then let the stage replace the row
                                    // with the real page and total when it opens.
                                    let settings_path = settings::path();
                                    let page = settings::resume_page(&settings_path, &path);
                                    let _ = settings::record_open(
                                        &settings_path,
                                        &path,
                                        page,
                                        None,
                                        next_opened_at(&settings_path),
                                    );
                                    notice_open.set(String::new());
                                    form::show_reading(&form_open);
                                    let mut current = visit_open.get();
                                    current.scroll = form::shelf_scroll();
                                    commit_visit(
                                        &visit_open,
                                        visit::open_cover(&current, &path, page),
                                    );
                                    if let Some(name) = cover_title(&library_open.get(), &path) {
                                        title_open.set(name);
                                    }
                                    variant_open.set(String::new());
                                    stage_open.command("open", json!(path));
                                    history_open.set(settings::load_history(&settings::path()));
                                },
                            )
                        }
                    },
                )),
        )
        .child(when(
            {
                let visit = visit.clone();
                move || shell_on && visit.get().surface == visit::Surface::Reading
            },
            {
                let form = shell_form.clone();
                let window = shell_window.clone();
                let chrome = shell_chrome.clone();
                let stage = stage.clone();
                move || {
                    form::reading_margin(
                        form.clone(),
                        window.clone(),
                        chrome.clone(),
                        stage.clone(),
                    )
                }
            },
        ))
        .child(when(move || show_text.get(), {
            let html = html.clone();
            move || {
                text_view()
                    .html(move || SharedString::from(html.get()))
                    .selectable(true)
                    .p_6()
                    .bg(hsla(40.0, 0.25, 0.97, 1.0))
            }
        }))
        .child(when(
            {
                let bars_row = bars_row.clone();
                let visit = visit.clone();
                move || visit::show_reader_chrome(shell_on, bars_row.get(), visit.get().surface)
            },
            {
                let left = left.clone();
                let right = right.clone();
                move || {
                    view()
                        .h(px(36.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(text(move || left.get()).text_size(px(13.0)))
                        .child(text(move || right.get()).text_size(px(13.0)))
                }
            },
        ))
}

fn deliver(arrival: &Signal<(u64, String)>, path: &Path) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let text = path.display().to_string();
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    arrival.set((seq, text));
}

fn listen_for_opens(arrival: Signal<(u64, String)>) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    spawn(async move {
        loop {
            for path in next_open_files().await {
                deliver(&arrival, &path);
            }
        }
    });
}

fn ready_book(path: &str) -> bool {
    let Some(book) = books::parse_book(path) else {
        return false;
    };
    if !books::can_open(path) {
        return false;
    }
    if book.kind == books::BookKind::Epub && Path::new(&book.path).is_dir() {
        return true;
    }
    let mut file = match std::fs::File::open(&book.path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    use std::io::Read;
    let mut buf = [0_u8; 4];
    let n = file.read(&mut buf).unwrap_or(0);
    visit::prefix_is_book(book.kind == books::BookKind::Pdf, &buf[..n])
}

fn receive_file(
    visit: &Signal<visit::Visit>,
    form: &Signal<form::Form>,
    notice: &Signal<String>,
    stage: &ElementRef,
    title: &Signal<String>,
    variant: &Signal<String>,
    history: &Signal<Vec<settings::ReadingRecord>>,
    path: &str,
) {
    let page = settings::resume_page(&settings::path(), path);
    let open = visit::file_open(ready_book(path), path, page);
    let visit::FileOpen::Read { path, page } = &open else {
        notice.set(visit::OPEN_REFUSAL.to_string());
        return;
    };
    notice.set(String::new());
    form::show_reading(form);
    let mut current = visit.get();
    if current.surface == visit::Surface::Library {
        current.scroll = form::shelf_scroll();
    }
    if let Some(name) = Path::new(path).file_stem().and_then(|stem| stem.to_str()) {
        title.set(name.to_string());
    }
    variant.set(String::new());
    let settings_path = settings::path();
    let _ = settings::record_open(
        &settings_path,
        path,
        *page,
        None,
        next_opened_at(&settings_path),
    );
    commit_visit(visit, visit::apply_file_open(&current, &open));
    match visit::stage_open(&open, stage.node().is_some()) {
        visit::StageOpen::Command => stage.command("open", json!(path)),
        visit::StageOpen::Queue => crate::launch::accept_path(Path::new(path)),
        visit::StageOpen::None => {}
    }
    history.set(settings::load_history(&settings::path()));
}

fn turn_page(stage: &ElementRef, visit: &Signal<visit::Visit>, shell: bool, forward: bool) {
    let delta = visit::page_key_delta(shell, visit.get().surface, forward);
    if delta != 0 {
        stage.command("page", json!(delta));
    }
}

fn commit_visit(slot: &Signal<visit::Visit>, next: visit::Visit) {
    visit::publish_surface(next.surface);
    slot.set(next);
}

fn apply_leave(
    visit: &Signal<visit::Visit>,
    form: &Signal<form::Form>,
    key: visit::LeaveKey,
    stage: &ElementRef,
) -> Option<bool> {
    let immersion = if matches!(form.get(), form::Form::Immersive) {
        visit::Immersion::On
    } else {
        visit::Immersion::Off
    };
    match visit::leave_key(key, visit::Focus::Idle, immersion, visit.get().surface) {
        visit::KeyEffect::LeaveBook => {
            let next = visit::leave_to_library(&visit.get());
            form::restore_shelf_scroll(next.scroll);
            form::show_shelf(form);
            commit_visit(visit, next);
            None
        }
        visit::KeyEffect::StayOnBook => {
            let reading = visit.get().surface == visit::Surface::Reading;
            form::apply_immersive(form, reading, true, stage)
        }
        visit::KeyEffect::None => None,
    }
}

fn cover_title(library: &form::LibraryIn, path: &str) -> Option<String> {
    library
        .continue_reading
        .iter()
        .chain(library.covers.iter())
        .find(|card| card.open_path.as_deref() == Some(path))
        .map(|card| card.title.clone())
}

fn choose_book(arrival: Signal<(u64, String)>) {
    spawn(async move {
        let dialog = OpenDialog::files().filter(FileFilter::new("Books", ["epub", "pdf"]));
        if let Ok(Some(paths)) = pick_paths(dialog).await {
            if let Some(path) = paths.into_iter().find_map(|path| {
                let text = path.display().to_string();
                books::parse_book(&text).map(|_| text)
            }) {
                deliver(&arrival, Path::new(&path));
            }
        }
    });
}

fn reading_menu(
    current: &str,
    bold: bool,
    needs_folder: bool,
    choose_library: impl Fn() + Clone + 'static,
    on_trim: impl Fn(String) + Clone + 'static,
    on_bold: impl Fn() + Clone + 'static,
    on_open: impl Fn() + Clone + 'static,
    immersive: bool,
    on_immersive: impl Fn() + Clone + 'static,
) -> Vec<MenuItem> {
    let mut menus = Vec::new();
    if os() == Os::MacOs {
        menus.push(MenuItem::submenu(
            "Slowpage",
            [
                MenuItem::role(MenuRole::About),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Hide),
                MenuItem::role(MenuRole::HideOthers),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Quit),
            ],
        ));
    }
    let mut file_items = vec![MenuItem::new("Open…").shortcut("mod+o").on_select(on_open)];
    if needs_folder {
        file_items.push(MenuItem::new("Choose Library Folder").on_select(choose_library));
    }
    menus.push(MenuItem::submenu("File", file_items));
    menus.push(MenuItem::submenu(
        "Edit",
        [
            MenuItem::role(MenuRole::Undo),
            MenuItem::role(MenuRole::Redo),
            MenuItem::separator(),
            MenuItem::role(MenuRole::Cut),
            MenuItem::role(MenuRole::Copy),
            MenuItem::role(MenuRole::Paste),
            MenuItem::role(MenuRole::SelectAll),
        ],
    ));
    menus.push(MenuItem::submenu(
        "Window",
        [
            MenuItem::role(MenuRole::Minimize),
            MenuItem::role(MenuRole::Zoom),
            MenuItem::role(MenuRole::ToggleFullScreen),
            MenuItem::separator(),
            MenuItem::role(MenuRole::Close),
        ],
    ));
    let mut view_items = vec![
        MenuItem::new("Immersive Reading")
            .shortcut(IMMERSIVE_KEY)
            .checked(immersive)
            .on_select(on_immersive),
        MenuItem::separator(),
    ];
    for item in reading::TRIMS {
        let id = item.id.to_string();
        let on_trim = on_trim.clone();
        view_items.push(
            MenuItem::new(item.label)
                .checked(current == item.id)
                .on_select(move || on_trim(id.clone())),
        );
    }
    view_items.push(MenuItem::separator());
    view_items.push(
        MenuItem::new(bold_word::menu_label())
            .checked(bold)
            .on_select(move || on_bold()),
    );
    menus.push(MenuItem::submenu("View", view_items));
    menus
}

#[cfg(test)]
mod launch_tests {
    use super::form;
    use super::visit::{self, PlaceFile, Surface, WheelOwner};
    use super::{apply_leave, launch_target, LaunchTarget};
    use nsh_core::element::reactive::signal;
    use nsh_core::prelude::element_ref;

    #[test]
    fn a_stored_page_does_not_open_the_shelf_launch() {
        assert_eq!(launch_target(false, false, Some(2)), LaunchTarget::Shelf);
        assert_eq!(launch_target(false, false, None), LaunchTarget::Shelf);
    }

    #[test]
    fn an_explicit_file_resumes_its_stored_page() {
        assert_eq!(
            launch_target(true, false, Some(2)),
            LaunchTarget::File { page: 2 }
        );
        assert_eq!(
            launch_target(true, true, Some(4)),
            LaunchTarget::File { page: 4 }
        );
        assert_eq!(
            launch_target(true, false, None),
            LaunchTarget::File { page: 1 }
        );
        assert_eq!(
            launch_target(true, false, Some(0)),
            LaunchTarget::File { page: 1 }
        );
    }

    #[test]
    fn a_reader_forcing_launch_is_not_the_shelf_and_does_not_take_the_place() {
        assert_eq!(launch_target(false, true, Some(2)), LaunchTarget::Forced);
    }

    #[test]
    fn a_direct_file_arms_the_wheel_without_the_shelf() {
        let file = launch_target(true, false, Some(4));
        assert_eq!(file, LaunchTarget::File { page: 4 });
        assert!(!file.engages_shell());
        assert!(launch_target(false, false, None).engages_shell());
        assert!(!launch_target(false, true, None).engages_shell());
        // These are the functions `sync` and the wheel monitor call.
        // This test does not run `sync` or install the monitor.
        visit::publish_surface(Surface::Library);
        assert!(visit::arms_page_wheel(false));
        assert_eq!(
            visit::wheel_owner(visit::wheel_scrolls_shelf(false), true),
            WheelOwner::Page
        );
        assert_eq!(
            visit::wheel_owner(visit::wheel_scrolls_shelf(false), false),
            WheelOwner::Through
        );
        assert_eq!(
            visit::wheel_owner(visit::wheel_scrolls_shelf(true), true),
            WheelOwner::Shelf
        );
        visit::publish_surface(Surface::Reading);
        assert_eq!(
            visit::wheel_owner(visit::wheel_scrolls_shelf(true), true),
            WheelOwner::Page
        );
        visit::publish_surface(Surface::Library);
    }

    #[test]
    fn immersive_reading_keeps_the_page_across_escape_resize_and_fullscreen() {
        let prior_margin = form::margin_shown();
        let prior_immersive = form::immersive_on();
        let prior_surface = visit::on_library();
        let dir =
            std::env::temp_dir().join(format!("superreader-immersive-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/field.pdf";
        struct Clean {
            dir: std::path::PathBuf,
            margin: bool,
            immersive: bool,
            library: bool,
        }
        impl Drop for Clean {
            fn drop(&mut self) {
                form::publish_margin(self.margin);
                form::publish_immersive(self.immersive);
                visit::publish_surface(if self.library {
                    Surface::Library
                } else {
                    Surface::Reading
                });
                let _ = std::fs::remove_dir_all(&self.dir);
            }
        }
        let _clean = Clean {
            dir: dir.clone(),
            margin: prior_margin,
            immersive: prior_immersive,
            library: prior_surface,
        };

        let chord = nsh_core::desktop::Shortcut::parse(super::IMMERSIVE_KEY, true).unwrap();
        assert!(chord.ctrl && chord.meta && !chord.alt && !chord.shift);
        assert_eq!(chord.key, nsh_core::desktop::Key::Char('i'));
        let _menu = nsh_core::element::desktop::MenuItem::new("Immersive Reading")
            .shortcut(super::IMMERSIVE_KEY);

        crate::settings::record_open(&file, book, 4, Some(80), 1).unwrap();
        crate::settings::record_progress(&file, book, 4, Some(80), 12, "ch.xhtml").unwrap();
        form::publish_margin(false);
        form::publish_immersive(false);

        let runtime = nsh_core::element::reactive::Runtime::default();
        let _entered = runtime.enter();
        let (owner, _) = nsh_core::element::reactive::create_root(|| {
            let stage = element_ref();
            let toggle =
                |form: &nsh_core::element::reactive::Signal<form::Form>,
                 visit: &nsh_core::element::reactive::Signal<visit::Visit>| {
                    form::apply_immersive(
                        form,
                        visit.get().surface == Surface::Reading,
                        false,
                        &stage,
                    )
                };
            let shelf = signal(visit::Visit::shelf());
            let form = signal(form::Form::PageAndPanel {
                sidebar: form::Presence::Hidden,
                margin: form::Presence::Hidden,
            });
            assert_eq!(toggle(&form, &shelf), None);
            assert!(matches!(
                form.get(),
                form::Form::PageAndPanel {
                    margin: form::Presence::Hidden,
                    ..
                }
            ));
            assert!(!form::immersive_on());
            assert_eq!(crate::settings::resume_page(&file, book), 4);
            assert_eq!(crate::settings::resume_offset(&file, book), 12);

            let visit = signal(visit::open_cover(&visit::Visit::shelf(), book, 4));
            assert_eq!(toggle(&form, &visit), Some(true));
            assert_eq!(form.get(), form::Form::Immersive);
            assert!(form::immersive_on());
            assert!(!form::margin_chrome(form.get()));
            assert_eq!(form::card_radius(true), 0.0);
            assert_eq!(form::card_radius(false), crate::tokens::CARD_RADIUS);

            let drawn =
                |w: f64, h: f64| form::page_frame(w, h, form::margin_shown(), form::immersive_on());
            assert_eq!(
                drawn(818.0, 800.0).card,
                form::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 818.0,
                    h: 800.0
                }
            );
            assert!(drawn(818.0, 800.0).margin.is_none());
            assert!(!drawn(818.0, 800.0).covers);
            assert_eq!(drawn(817.0, 800.0).card.w, 817.0);
            assert_eq!(
                drawn(1440.0, 900.0).card,
                form::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 1440.0,
                    h: 900.0
                }
            );

            let library = form::LibraryIn {
                empty: true,
                book_count: 0,
                counts: form::LibraryCounts {
                    all: 0,
                    reading: 0,
                    want: 0,
                    done: 0,
                    pdf: 0,
                    notes: 0,
                },
                sources: Vec::new(),
                selected: form::SelectId::AllBooks,
                continue_reading: Vec::new(),
                covers: Vec::new(),
                lists: Vec::new(),
                chat: Vec::new(),
                notice: None,
            };
            for (width, height, top) in [(900.0, 700.0, 28.0), (1440.0, 900.0, 0.0)] {
                let placed = form::place(
                    &form::PlaceIn {
                        form: form.get(),
                        window: form::Window { width, height },
                        chrome: crate::tokens::ChromeIn {
                            lights: true,
                            bar: 92.0,
                            top,
                        },
                    },
                    &library,
                );
                assert!(!placed.gray);
                assert!(placed.sidebar.is_none());
                assert!(placed.chat.is_none());
                let form::CardBox::Placed {
                    rect,
                    radius,
                    shadow,
                } = placed.card
                else {
                    panic!("card");
                };
                assert_eq!((rect.x, rect.y, rect.w, rect.h), (0.0, 0.0, width, height));
                assert_eq!(radius, 0.0);
                assert!(!shadow);
            }
            assert_eq!(crate::reading::title_alpha(0), 1.0);
            assert_eq!(crate::reading::title_alpha(900), 0.0);

            let mut reader = visit::Reader::default();
            let store = crate::settings::SettingsBook { file: &file, book };
            visit::open_resume(&mut reader, &store);
            let mut enter_tick = visit::FrameTick::new(false, true, true, true, true, false);
            enter_tick.shown = 4;
            enter_tick.pages = 20;
            enter_tick.pages_known = true;
            assert_eq!(
                visit::place_step(false, true, true),
                visit::PlaceStep::Cancel { reflow: true }
            );
            assert_eq!(
                visit::layout_frame(&mut reader, enter_tick).draw,
                visit::Draw::Move { reveal: Some(12) }
            );
            let mut resize_tick = visit::FrameTick::new(false, true, false, true, true, false);
            resize_tick.shown = 4;
            resize_tick.pages = 20;
            resize_tick.pages_known = true;
            assert_eq!(
                visit::layout_frame(&mut reader, resize_tick).draw,
                visit::Draw::Move { reveal: Some(12) }
            );
            assert!(visit::landed_progress(4, 12, 4).is_none());
            assert_eq!(store.resume().page, 4);
            assert_eq!(store.resume().offset, 12);

            form::show_reading(&form);
            assert!(!form::immersive_on());
            assert!(matches!(
                form.get(),
                form::Form::PageAndPanel {
                    sidebar: form::Presence::Hidden,
                    margin: form::Presence::Hidden,
                }
            ));
            assert_eq!(store.resume().page, 4);

            assert_eq!(toggle(&form, &visit), Some(true));
            assert_eq!(toggle(&form, &visit), Some(false));
            assert!(matches!(
                form.get(),
                form::Form::PageAndPanel {
                    sidebar: form::Presence::Hidden,
                    margin: form::Presence::Shown,
                }
            ));
            assert!(form::margin_shown());
            assert_eq!(toggle(&form, &visit), Some(true));
            assert_eq!(form.get(), form::Form::Immersive);
            assert_eq!(store.resume().page, 4);
            assert_eq!(store.resume().offset, 12);

            let hit = |width: f64, height: f64, x_at: f64| {
                let frame = form::stage_card(width, height, true);
                let (x, y, w, h) = spike_report::fitted_pdf_page(
                    frame.card.x,
                    frame.card.y,
                    frame.card.w,
                    frame.card.h,
                    612.0,
                    792.0,
                );
                let point = x + w * x_at;
                let immersive = visit::page_edge(point, y + h / 2.0, x, y, w, h);
                let old = form::reading_frame(width, height, 0.0, 0.0, true);
                let (ox, oy, ow, oh) = spike_report::fitted_pdf_page(
                    old.card.x, old.card.y, old.card.w, old.card.h, 612.0, 792.0,
                );
                let (ox, spanned) = form::visible_page_span(ox, ow, &old);
                let stale = visit::page_edge(point, y + h / 2.0, ox, oy, spanned, oh);
                (frame, immersive, stale)
            };
            let (wide, wide_edge, wide_stale) = hit(900.0, 700.0, 0.9);
            assert_eq!(
                wide.card,
                form::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 900.0,
                    h: 700.0
                }
            );
            assert_eq!(wide_edge, visit::PageEdge::Forward);
            assert_eq!(wide_stale, visit::PageEdge::None);
            let (narrow, narrow_edge, narrow_stale) = hit(700.0, 800.0, 0.9);
            assert_eq!(narrow.card.w, 700.0);
            assert_eq!(narrow_edge, visit::PageEdge::Forward);
            assert_eq!(narrow_stale, visit::PageEdge::None);
            let bare = form::stage_card(900.0, 700.0, false);
            assert!((bare.card.x - crate::tokens::INSET).abs() < 0.01);
            assert!((bare.card.w - (900.0 - crate::tokens::INSET * 2.0)).abs() < 0.01);

            assert_eq!(
                apply_leave(&visit, &form, visit::LeaveKey::Back, &stage),
                None
            );
            assert_eq!(form.get(), form::Form::Immersive);
            assert_eq!(visit.get().surface, Surface::Reading);
            assert_eq!(visit.get().page, 4);

            assert_eq!(
                apply_leave(&visit, &form, visit::LeaveKey::Escape, &stage),
                Some(false)
            );
            assert!(matches!(
                form.get(),
                form::Form::PageAndPanel {
                    sidebar: form::Presence::Hidden,
                    margin: form::Presence::Shown,
                }
            ));
            assert!(!form::immersive_on());
            assert!(form::margin_chrome(form.get()));
            assert_eq!(visit.get().surface, Surface::Reading);
            assert_eq!(visit.get().page, 4);
            let column = drawn(818.0, 800.0);
            let hidden = form::page_frame(818.0, 800.0, false, false);
            assert!((hidden.card.w - column.card.w - form::MARGIN_W).abs() < 0.1);
            assert!(!column.covers);
            let covered = drawn(817.0, 800.0);
            let covered_hidden = form::page_frame(817.0, 800.0, false, false);
            assert_eq!(covered.card, covered_hidden.card);
            assert!(covered.covers);
            let reopened = crate::settings::SettingsBook { file: &file, book };
            assert_eq!(reopened.resume().page, 4);
            assert_eq!(reopened.resume().offset, 12);

            assert_eq!(
                apply_leave(&visit, &form, visit::LeaveKey::Escape, &stage),
                None
            );
            assert_eq!(visit.get().surface, Surface::Library);
            assert_eq!(form.get(), form::launch());
            assert!(!form::immersive_on());
            assert_eq!(toggle(&form, &visit), None);
            assert_eq!(form.get(), form::launch());
            assert_eq!(reopened.resume().page, 4);

            let visit = signal(visit::open_cover(&visit::Visit::shelf(), book, 4));
            form.set(form::Form::PageAndPanel {
                sidebar: form::Presence::Hidden,
                margin: form::Presence::Shown,
            });
            assert_eq!(toggle(&form, &visit), Some(true));
            assert_eq!(
                form::step(form.get(), form::Command::ToggleSidebar),
                form::Form::Immersive
            );
            assert_eq!(
                form::step(form.get(), form::Command::TogglePanel),
                form::Form::Immersive
            );
            assert_eq!(reopened.resume().page, 4);
            assert_eq!(reopened.resume().offset, 12);
        });
        owner.dispose();
    }
}
