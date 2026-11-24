//! The shell is one form. `step` is the only writer. `place` is the geometry.
//! `mount` paints that geometry and does not invent a second one.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use nsh_core::prelude::*;

const SIDEBAR_W: f64 = crate::tokens::SIDEBAR_W;
const CHAT_W: f64 = crate::tokens::CHAT_W;
const INSET: f64 = crate::tokens::INSET;
const RADIUS: f64 = crate::tokens::CARD_RADIUS;

pub const FIELD_HEX: u32 = crate::tokens::FIELD_TOP;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presence {
    Shown,
    Hidden,
}

impl Presence {
    fn flip(self) -> Self {
        match self {
            Presence::Shown => Presence::Hidden,
            Presence::Hidden => Presence::Shown,
        }
    }

    fn shown(self) -> bool {
        self == Presence::Shown
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Full { sidebar: Presence, chat: Presence },
    PageAndPanel { sidebar: Presence, margin: Presence },
    Immersive,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    ToggleSidebar,
    TogglePanel,
    ToggleImmersive,
    LeaveImmersive,
}

pub fn launch() -> Form {
    Form::Full {
        sidebar: Presence::Shown,
        chat: Presence::Shown,
    }
}

pub fn step(form: Form, command: Command) -> Form {
    match (form, command) {
        (Form::PageAndPanel { .. }, Command::ToggleImmersive) => Form::Immersive,
        (Form::Immersive, Command::ToggleImmersive | Command::LeaveImmersive) => {
            Form::PageAndPanel {
                sidebar: Presence::Hidden,
                margin: Presence::Shown,
            }
        }
        (Form::Immersive, _) => Form::Immersive,
        (_, Command::ToggleImmersive | Command::LeaveImmersive) => form,
        (Form::Full { sidebar, chat }, Command::ToggleSidebar) => Form::Full {
            sidebar: sidebar.flip(),
            chat,
        },
        (Form::Full { sidebar, chat }, Command::TogglePanel) => Form::Full {
            sidebar,
            chat: chat.flip(),
        },
        (Form::PageAndPanel { sidebar, margin }, Command::ToggleSidebar) => Form::PageAndPanel {
            sidebar: sidebar.flip(),
            margin,
        },
        (Form::PageAndPanel { sidebar, margin }, Command::TogglePanel) => Form::PageAndPanel {
            sidebar,
            margin: margin.flip(),
        },
    }
}

fn reading_immersive(form: Form, reading: bool, leave_only: bool) -> Form {
    if !reading {
        return form;
    }
    step(
        form,
        if leave_only {
            Command::LeaveImmersive
        } else {
            Command::ToggleImmersive
        },
    )
}

/// The native stage. `ElementRef::command` is what marks the view dirty.
/// NSH 0.5.1 does not let a unit test install the executor `element_ref`
/// needs, so a test supplies this and the window supplies the real ref.
pub trait StageCommand {
    fn stage_command(&self, name: &str, args: nsh_core::JsonValue);
}

impl StageCommand for ElementRef {
    fn stage_command(&self, name: &str, args: nsh_core::JsonValue) {
        self.command(name, args);
    }
}

/// The shortcut, the View menu, and Escape call this.
/// An unchanged form does not touch the published flags.
/// A change sends `immersive` to the stage, the same redraw a command gets.
pub fn apply_immersive(
    form: &Signal<Form>,
    reading: bool,
    leave_only: bool,
    stage: &impl StageCommand,
) -> Option<bool> {
    let before = form.get();
    let next = commit_reading_immersive(before, reading, leave_only);
    form.set(next);
    let notice = (next != before).then_some(matches!(next, Form::Immersive));
    if let Some(on) = notice {
        let margin = match next {
            Form::PageAndPanel { margin, .. } => margin.shown(),
            _ => margin_shown(),
        };
        crate::ease::retarget(on, margin);
        stage.stage_command("immersive", serde_json::json!(on));
    }
    notice
}

pub fn book_stays_for_escape(form: Form) -> bool {
    crate::ease::escape_stays(matches!(form, Form::Immersive), crate::ease::current())
}

fn commit_reading_immersive(form: Form, reading: bool, leave_only: bool) -> Form {
    let next = reading_immersive(form, reading, leave_only);
    if next == form {
        return form;
    }
    match next {
        Form::Immersive => publish_immersive(true),
        Form::PageAndPanel { margin, .. } => {
            publish_immersive(false);
            publish_margin(margin.shown());
        }
        Form::Full { .. } => publish_immersive(false),
    }
    next
}

pub fn margin_chrome(form: Form) -> bool {
    matches!(form, Form::PageAndPanel { .. })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryCounts {
    pub all: u32,
    pub reading: u32,
    pub want: u32,
    pub done: u32,
    pub pdf: u32,
    pub notes: u32,
}

impl Default for LibraryCounts {
    fn default() -> Self {
        Self {
            all: 0,
            reading: 0,
            want: 0,
            done: 0,
            pdf: 0,
            notes: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIn {
    pub label: String,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverCard {
    pub title: String,
    pub author: String,
    pub cover: Option<PathBuf>,
    pub open_path: Option<String>,
    pub pdf: bool,
    pub percent: Option<u8>,
    pub page: Option<u32>,
    pub position: String,
    pub opened: String,
    /// No photo cover. The shelf draws the title and author on the plate.
    pub plate: bool,
    /// Fixture plate color from the cover file, when the PNG carries one.
    pub plate_rgb: Option<u32>,
    pub unavailable: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookList {
    pub label: String,
    pub count: u32,
    pub color: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LibraryIn {
    pub empty: bool,
    pub book_count: u32,
    pub counts: LibraryCounts,
    pub sources: Vec<SourceIn>,
    pub selected: SelectId,
    pub continue_reading: Vec<CoverCard>,
    pub covers: Vec<CoverCard>,
    pub lists: Vec<BookList>,
    pub chat: Vec<ChatLine>,
    pub notice: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SelectId {
    AllBooks,
    Reading,
    Want,
    Done,
    Pdf,
    Notes,
    AddSource,
    Settings,
    Source(String),
}

impl Default for SelectId {
    fn default() -> Self {
        Self::AllBooks
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceIn {
    pub form: Form,
    pub window: Window,
    pub chrome: crate::tokens::ChromeIn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub gray: bool,
    pub sidebar: Option<Rect>,
    pub card: CardBox,
    pub chat: Option<Rect>,
    pub chrome: crate::tokens::ChromeIn,
    pub face: CardFace,
    pub rows: Vec<SideRow>,
    pub groups: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardBox {
    Placed {
        rect: Rect,
        radius: f64,
        shadow: bool,
    },
}

/// The annotation column. The chat panel's 320 points are not this width.
pub const MARGIN_W: f64 = 240.0;

static MARGIN_SHOWN: AtomicBool = AtomicBool::new(true);
static FIND_ESC: AtomicBool = AtomicBool::new(false);

pub fn clear_find_escape() {
    FIND_ESC.store(false, Ordering::Relaxed);
}

pub fn hold_find_escape() {
    FIND_ESC.store(true, Ordering::Relaxed);
}

pub fn take_find_escape() -> bool {
    FIND_ESC.swap(false, Ordering::Relaxed)
}

pub fn show_reading_margin(form: Form) -> Form {
    match form {
        Form::PageAndPanel {
            sidebar,
            margin: Presence::Hidden,
        } => Form::PageAndPanel {
            sidebar,
            margin: Presence::Shown,
        },
        other => other,
    }
}

pub fn margin_shown() -> bool {
    MARGIN_SHOWN.load(Ordering::Relaxed)
}

pub fn publish_margin(shown: bool) {
    MARGIN_SHOWN.store(shown, Ordering::Relaxed);
    crate::ease::follow_margin(shown);
}

static IMMERSIVE: AtomicBool = AtomicBool::new(false);

pub fn immersive_on() -> bool {
    IMMERSIVE.load(Ordering::Relaxed)
}

pub fn publish_immersive(on: bool) {
    IMMERSIVE.store(on, Ordering::Relaxed);
}

/// A shown margin is a column once the window can hold the minimum plus 240.
pub fn margin_is_column(window_w: f64) -> bool {
    window_w >= crate::tokens::MIN_WINDOW_W + MARGIN_W
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingFrame {
    pub card: Rect,
    pub margin: Option<Rect>,
    pub covers: bool,
}

/// The page card and, when the margin is shown, the right region.
/// Below the column width the right region covers the card. The card does not change.
pub fn reading_frame(width: f64, height: f64, top: f64, sidebar: f64, shown: bool) -> ReadingFrame {
    let top = top.max(0.0);
    let sidebar = sidebar.max(0.0);
    let card = Rect {
        x: sidebar + INSET,
        y: top + INSET,
        w: (width - sidebar - INSET * 2.0).max(1.0),
        h: (height - top - INSET * 2.0).max(1.0),
    };
    if !shown {
        return ReadingFrame {
            card,
            margin: None,
            covers: false,
        };
    }
    let margin = Rect {
        x: (width - MARGIN_W).max(0.0),
        y: 0.0,
        w: MARGIN_W.min(width.max(0.0)),
        h: height.max(1.0),
    };
    if margin_is_column(width) {
        ReadingFrame {
            card: Rect {
                w: (card.w - MARGIN_W).max(1.0),
                ..card
            },
            margin: Some(margin),
            covers: false,
        }
    } else {
        ReadingFrame {
            card,
            margin: Some(margin),
            covers: true,
        }
    }
}

pub fn margin_claims_wheel(x: f64, y: f64, margin: Option<Rect>) -> bool {
    let Some(rect) = margin else {
        return false;
    };
    rect.w > 0.5
        && rect.h > 0.5
        && x >= rect.x
        && y >= rect.y
        && x < rect.x + rect.w
        && y < rect.y + rect.h
}

/// The fitted page a click can reach. A covering margin removes the part it
/// covers. A column leaves the fitted page, because the card already sits
/// beside the margin.
pub fn visible_page_span(page_x: f64, page_w: f64, frame: &ReadingFrame) -> (f64, f64) {
    if !frame.covers {
        return (page_x, page_w);
    }
    let Some(margin) = frame.margin else {
        return (page_x, page_w);
    };
    let page_right = page_x + page_w;
    if page_right <= margin.x || page_x >= margin.x + margin.w {
        return (page_x, page_w);
    }
    let right = page_right.min(margin.x);
    (page_x, (right - page_x).max(0.0))
}

/// The card meets every window edge. The margin is not on screen.
pub fn immersive_frame(width: f64, height: f64) -> ReadingFrame {
    ReadingFrame {
        card: Rect {
            x: 0.0,
            y: 0.0,
            w: width.max(1.0),
            h: height.max(1.0),
        },
        margin: None,
        covers: false,
    }
}

/// The card the stage draws. Immersive reading uses the window. Otherwise the
/// page card keeps the 6 point inset and the margin rule.
pub fn page_frame(width: f64, height: f64, margin: bool, immersive: bool) -> ReadingFrame {
    if immersive {
        immersive_frame(width, height)
    } else {
        reading_frame(width, height, 0.0, 0.0, margin)
    }
}

/// The card the stage paints and the card a click tests. `shell` is the
/// reading window. The old bar ignores the immersive flag.
pub fn stage_card(width: f64, height: f64, shell: bool) -> ReadingFrame {
    page_frame(
        width,
        height,
        shell && margin_shown(),
        shell && immersive_on(),
    )
}

pub fn card_radius(immersive: bool) -> f64 {
    if immersive {
        0.0
    } else {
        crate::tokens::CARD_RADIUS
    }
}

pub fn show_reading(form: &Signal<Form>) {
    let margin = if margin_shown() {
        Presence::Shown
    } else {
        Presence::Hidden
    };
    form.set(Form::PageAndPanel {
        sidebar: Presence::Hidden,
        margin,
    });
    publish_immersive(false);
    crate::ease::settle(false, margin.shown());
}

pub fn show_shelf(form: &Signal<Form>) {
    form.set(launch());
    publish_immersive(false);
    crate::ease::settle(false, false);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFace {
    EmptyPeers,
    Grid,
    Placeholder,
    Page,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideRow {
    pub id: SelectId,
    pub label: String,
    pub count: Option<u32>,
    pub icon: &'static str,
    pub selected: bool,
    pub group: &'static str,
    pub dot: Option<u32>,
    pub badge: Option<(u32, u32)>,
}

pub fn place(input: &PlaceIn, library: &LibraryIn) -> Placement {
    let window = input.window;
    let row = crate::tokens::chrome_row(input.chrome);
    let (sidebar_w, chat_w, inset, radius, shadow, gray) = match input.form {
        Form::Full { sidebar, chat } => {
            let sidebar_w = if sidebar.shown() { SIDEBAR_W } else { 0.0 };
            let chat_w = if chat.shown()
                && crate::tokens::chat_fits(window.width as f32, sidebar_w as f32)
            {
                CHAT_W
            } else {
                0.0
            };
            (sidebar_w, chat_w, INSET, RADIUS, true, true)
        }
        Form::PageAndPanel { sidebar, margin } => {
            let sidebar_w = if sidebar.shown() { SIDEBAR_W } else { 0.0 };
            let frame = reading_frame(
                window.width,
                window.height,
                f64::from(input.chrome.top.max(0.0)),
                sidebar_w,
                margin.shown(),
            );
            return Placement {
                gray: true,
                sidebar: sidebar_rect(sidebar.shown(), window),
                card: CardBox::Placed {
                    rect: frame.card,
                    radius: RADIUS,
                    shadow: true,
                },
                chat: None,
                chrome: input.chrome,
                face: CardFace::Page,
                rows: rows(library),
                groups: groups(),
            };
        }
        Form::Immersive => {
            return Placement {
                gray: false,
                sidebar: None,
                card: CardBox::Placed {
                    rect: immersive_frame(window.width, window.height).card,
                    radius: 0.0,
                    shadow: false,
                },
                chat: None,
                chrome: input.chrome,
                face: CardFace::Page,
                rows: rows(library),
                groups: groups(),
            };
        }
    };
    let top = f64::from(input.chrome.top.max(0.0));
    let rect = Rect {
        x: sidebar_w + inset,
        y: top + inset,
        w: (window.width - sidebar_w - chat_w - inset * 2.0).max(1.0),
        h: (window.height - top - inset * 2.0).max(1.0),
    };
    let face = match input.form {
        Form::Full { .. } => {
            if matches!(library.selected, SelectId::Settings) {
                CardFace::Placeholder
            } else if library.empty {
                CardFace::EmptyPeers
            } else {
                CardFace::Grid
            }
        }
        _ => CardFace::Page,
    };
    Placement {
        gray,
        sidebar: sidebar_rect(sidebar_w > 0.0, window),
        card: CardBox::Placed {
            rect,
            radius,
            shadow,
        },
        chat: if chat_w > 0.0 {
            Some(Rect {
                x: window.width - chat_w,
                y: f64::from(row.card_y - INSET as f32),
                w: chat_w,
                h: (window.height - f64::from(row.card_y - INSET as f32)).max(1.0),
            })
        } else {
            None
        },
        chrome: input.chrome,
        face,
        rows: rows(library),
        groups: groups(),
    }
}

fn sidebar_rect(shown: bool, window: Window) -> Option<Rect> {
    if !shown {
        return None;
    }
    Some(Rect {
        x: 0.0,
        y: 0.0,
        w: SIDEBAR_W,
        h: window.height,
    })
}

fn groups() -> Vec<&'static str> {
    vec!["书库", "笔记", "书单", "来源"]
}

fn rows(library: &LibraryIn) -> Vec<SideRow> {
    let c = &library.counts;
    let mut out = vec![
        row(
            SelectId::AllBooks,
            "全部书籍",
            Some(c.all),
            "library",
            "书库",
            &library.selected,
        ),
        row(
            SelectId::Reading,
            "正在读",
            Some(c.reading),
            "book-open",
            "书库",
            &library.selected,
        ),
        row(
            SelectId::Want,
            "想读",
            Some(c.want),
            "bookmark",
            "书库",
            &library.selected,
        ),
        row(
            SelectId::Done,
            "已读完",
            Some(c.done),
            "check",
            "书库",
            &library.selected,
        ),
        row(
            SelectId::Pdf,
            "PDF",
            Some(c.pdf),
            "file",
            "书库",
            &library.selected,
        ),
        row(
            SelectId::Notes,
            "高亮与笔记",
            Some(c.notes),
            "note",
            "笔记",
            &library.selected,
        ),
    ];
    for list in &library.lists {
        let mut item = row(
            SelectId::Source(list.label.clone()),
            &list.label,
            Some(list.count),
            "",
            "书单",
            &library.selected,
        );
        item.dot = Some(list.color);
        out.push(item);
    }
    for source in &library.sources {
        let (icon, badge) = source_face(&source.label);
        let mut item = row(
            SelectId::Source(source.label.clone()),
            &source.label,
            Some(source.count),
            icon,
            "来源",
            &library.selected,
        );
        item.badge = badge;
        out.push(item);
    }
    out.push(row(
        SelectId::AddSource,
        "添加来源",
        None,
        "plus",
        "来源",
        &library.selected,
    ));
    out.push(row(
        SelectId::Settings,
        "设置",
        None,
        "sliders",
        "",
        &library.selected,
    ));
    out
}

fn source_face(label: &str) -> (&'static str, Option<(u32, u32)>) {
    match label {
        "Apple Books" => (
            "book-open",
            Some((crate::tokens::APPLE_FROM, crate::tokens::APPLE_TO)),
        ),
        "calibre" => (
            "library",
            Some((crate::tokens::CALIBRE_FROM, crate::tokens::CALIBRE_TO)),
        ),
        _ => ("folder", None),
    }
}

/// A connected source keeps its count. A denied import drops that label.
pub fn remember_source(rows: &mut Vec<SourceIn>, label: &str, count: u32, connected: bool) {
    if !connected {
        rows.retain(|row| row.label != label);
        return;
    }
    if let Some(row) = rows.iter_mut().find(|row| row.label == label) {
        row.count = count;
        return;
    }
    rows.push(SourceIn {
        label: label.to_string(),
        count,
    });
}

fn row(
    id: SelectId,
    label: &str,
    count: Option<u32>,
    icon: &'static str,
    group: &'static str,
    selected: &SelectId,
) -> SideRow {
    SideRow {
        selected: &id == selected,
        id,
        label: label.to_string(),
        count,
        icon,
        group,
        dot: None,
        badge: None,
    }
}

pub fn selected_title(id: &SelectId) -> String {
    match id {
        SelectId::AllBooks => "全部书籍".into(),
        SelectId::Reading => "正在读".into(),
        SelectId::Want => "想读".into(),
        SelectId::Done => "已读完".into(),
        SelectId::Pdf => "PDF".into(),
        SelectId::Notes => "高亮与笔记".into(),
        SelectId::AddSource => "添加来源".into(),
        SelectId::Settings => "设置".into(),
        SelectId::Source(label) => label.clone(),
    }
}

fn ink() -> Hsla {
    rgb(crate::tokens::INK).to_hsla()
}

fn quiet() -> Hsla {
    rgb(crate::tokens::QUIET).to_hsla()
}

fn icon_ink() -> Hsla {
    rgb(crate::tokens::ICON).to_hsla()
}

fn faint() -> Hsla {
    rgb(crate::tokens::FAINT).to_hsla()
}

fn wash(alpha: f32) -> Hsla {
    hsla(0.0, 0.0, 0.0, alpha)
}

macro_rules! line {
    ($text:expr, $size:expr, $weight:expr, $color:expr, $track:expr) => {
        text($text)
            .font_family(SharedString::from(crate::tokens::FONT_STACK))
            .text_size(px($size))
            .font_weight($weight)
            .text_color($color)
            .letter_spacing(px(crate::tokens::track($size, $track)))
    };
}

pub fn chrome_off() -> crate::tokens::ChromeIn {
    crate::tokens::ChromeIn {
        lights: false,
        bar: 0.0,
        top: 0.0,
    }
}

pub fn mount(
    form: Signal<Form>,
    selected: Signal<SelectId>,
    window: Signal<Window>,
    chrome: Signal<crate::tokens::ChromeIn>,
    library: Signal<LibraryIn>,
    menu: Signal<bool>,
    on_peer: impl Fn(Peer) + Clone + 'static,
    on_open: impl Fn(String) + Clone + 'static,
) -> impl IntoElement {
    let form_b = form;
    let selected_b = selected;
    let window_b = window;
    let chrome_b = chrome;
    let library_b = library;
    let menu_b = menu;
    let on_peer_b = on_peer;
    let on_open_b = on_open;
    let query = signal(String::new());
    dynamic(move || {
        let current = form_b.get();
        let query = query.clone();
        let library = crate::shelf::searched(library_b.get(), &query.get());
        let placed = place(
            &PlaceIn {
                form: current,
                window: window_b.get(),
                chrome: chrome_b.get(),
            },
            &library,
        );
        write_layout(&placed, &library);
        paint(
            placed,
            form_b,
            selected_b,
            menu_b,
            query,
            library,
            on_peer_b.clone(),
            on_open_b.clone(),
        )
    })
}

struct FontNote {
    label: String,
    size: f32,
    weight: u16,
}

fn font_notes() -> &'static std::sync::Mutex<Vec<FontNote>> {
    static NOTES: std::sync::OnceLock<std::sync::Mutex<Vec<FontNote>>> = std::sync::OnceLock::new();
    NOTES.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn fonts_clear() {
    if let Ok(mut notes) = font_notes().lock() {
        notes.clear();
    }
}

fn remember_font(label: &str, size: f32, weight: FontWeight) {
    if let Ok(mut notes) = font_notes().lock() {
        if notes.iter().any(|note| {
            note.label == label && (note.size - size).abs() < 0.1 && note.weight == weight.0
        }) {
            return;
        }
        notes.push(FontNote {
            label: label.to_string(),
            size,
            weight: weight.0,
        });
    }
}

thread_local! {
    static SHELF_SCROLL: RefCell<Option<ElementRef>> = const { RefCell::new(None) };
    static SHELF_OFFSET: Cell<f32> = const { Cell::new(0.0) };
    static SHELF_HIT: Cell<[f32; 4]> = const { Cell::new([0.0, 0.0, 0.0, 0.0]) };
}

fn note_shelf_hit(frame: [f32; 4]) {
    SHELF_HIT.with(|hit| hit.set(frame));
}

/// Window coordinates, origin at the top left. A wheel inside this rectangle
/// scrolls the shelf. A wheel outside it is left for the sidebar or the chat.
pub fn shelf_hit_contains(x: f32, y: f32) -> bool {
    let [left, top, width, height] = SHELF_HIT.with(|hit| hit.get());
    width > 1.0 && height > 1.0 && x >= left && y >= top && x < left + width && y < top + height
}

fn remember_shelf(shelf: &ElementRef) {
    SHELF_SCROLL.with(|slot| *slot.borrow_mut() = Some(shelf.clone()));
}

pub fn shelf_scroll() -> f32 {
    SHELF_OFFSET.with(|stored| stored.get())
}

pub fn restore_shelf_scroll(offset: f32) {
    SHELF_OFFSET.with(|stored| stored.set(crate::visit::shelf_origin(offset)));
}

fn note_shelf_offset(offset: f32) {
    if offset.is_finite() {
        SHELF_OFFSET.with(|stored| stored.set(crate::visit::shelf_origin(offset)));
    }
}

/// A wheel that reached the stage instead of the shelf. Positive moves down.
pub fn nudge_shelf(delta: f32) {
    if !delta.is_finite() || delta.abs() < 0.01 {
        return;
    }
    let next = SHELF_OFFSET.with(|stored| {
        let next = (stored.get() + delta).max(0.0);
        stored.set(next);
        next
    });
    SHELF_SCROLL.with(|slot| {
        if let Some(shelf) = slot.borrow().as_ref() {
            shelf.scroll_to(point(px(0.0), px(next)), false);
        }
    });
}

fn fonts_write() {
    let Ok(notes) = font_notes().lock() else {
        return;
    };
    let body = serde_json::json!(notes
        .iter()
        .map(|note| serde_json::json!({
            "label": note.label,
            "size": note.size,
            "weight": note.weight,
        }))
        .collect::<Vec<_>>());
    let dir = "/tmp/superreader-spike";
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(format!("{dir}/fonts.json"), body.to_string());
}

fn fit_text(
    label: String,
    size: f32,
    weight: FontWeight,
    color: Hsla,
    tracking: f32,
) -> AnyElement {
    remember_font(&label, size, weight);
    view()
        .min_w(px(0.0))
        .tooltip(text(label.clone()).text_size(px(size)).text_color(color))
        .child(
            line!(label.clone(), size, weight, color, tracking)
                .truncate()
                .accessible(true)
                .label(label),
        )
        .into_element()
}

fn write_layout(placed: &Placement, library: &LibraryIn) {
    let CardBox::Placed { rect, .. } = placed.card;
    let card = serde_json::json!([rect.x, rect.y, rect.w, rect.h]);
    let sidebar = placed.sidebar.map(|rect| rect.w).unwrap_or(0.0);
    let chat = placed.chat.map(|rect| rect.w).unwrap_or(0.0);
    let body = serde_json::json!({
        "sidebar_w": sidebar,
        "chat_w": chat,
        "card": card,
        "gray": placed.gray,
    });
    let path = std::path::Path::new("/tmp/superreader-spike/shell-layout.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, body.to_string());
    write_frames(placed, library);
}

fn text_width(label: &str, size: f32) -> f32 {
    label
        .chars()
        .map(|ch| if ch.is_ascii() { size * 0.56 } else { size })
        .sum()
}

fn layout_node(
    id: &str,
    kind: &str,
    frame: [f32; 4],
    intrinsic: [f32; 2],
    container: [f32; 4],
    ellipsis: bool,
    label: &str,
    scroll_child: bool,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "kind": kind,
        "frame": frame,
        "intrinsic": intrinsic,
        "container": container,
        "ellipsis": ellipsis,
        "label": label,
        "scroll_child": scroll_child,
    })
}

fn layout_nodes(placed: &Placement, library: &LibraryIn) -> Vec<serde_json::Value> {
    let CardBox::Placed { rect, .. } = placed.card else {
        return Vec::new();
    };
    let window_w = if let Some(chat) = placed.chat {
        (chat.x + chat.w) as f32
    } else {
        (rect.x + rect.w + INSET) as f32
    };
    let window_h = (rect.y + rect.h + INSET) as f32;
    let window = [0.0_f32, 0.0, window_w, window_h];
    let content_w = (rect.w as f32 - crate::tokens::CARD_PAD_X * 2.0).max(0.0);
    let origin_x = rect.x as f32 + crate::tokens::CARD_PAD_X;
    let content = [origin_x, rect.y as f32, content_w, rect.h as f32];
    let mut nodes = vec![layout_node(
        "card",
        "container",
        [rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32],
        [rect.w as f32, rect.h as f32],
        window,
        false,
        "",
        false,
    )];
    let chrome = crate::tokens::chrome_row(placed.chrome);
    if let Some(side) = placed.sidebar {
        let side_frame = [side.x as f32, side.y as f32, side.w as f32, side.h as f32];
        nodes.push(layout_node(
            "sidebar",
            "container",
            side_frame,
            [side.w as f32, side.h as f32],
            window,
            false,
            "",
            false,
        ));
        let search_w = (side.w as f32 - crate::tokens::SIDE_PAD_X * 2.0).max(0.0);
        nodes.push(layout_node(
            "sidebar.search",
            "container",
            [
                side.x as f32 + crate::tokens::SIDE_PAD_X,
                side.y as f32 + chrome.height,
                search_w,
                crate::tokens::SEARCH_H,
            ],
            [search_w, crate::tokens::SEARCH_H],
            side_frame,
            false,
            "搜索",
            false,
        ));
        let row_w = (side.w as f32 - crate::tokens::SIDE_PAD_X * 2.0).max(0.0);
        let scroll_top = chrome.height + crate::tokens::SEARCH_MB + crate::tokens::SEARCH_H;
        let settings_h = crate::tokens::ROW_H + crate::tokens::SIDE_PAD_BOTTOM;
        let settings_y = (side.h as f32 - settings_h).max(scroll_top);
        let viewport = [
            side.x as f32,
            scroll_top,
            side.w as f32,
            (settings_y - scroll_top).max(0.0),
        ];
        let mut y = scroll_top;
        let mut index = 0u32;
        for label in &placed.groups {
            let label_h =
                crate::tokens::LABEL_MT + crate::tokens::LABEL_SIZE + crate::tokens::LABEL_MB;
            if let Some(frame) =
                clip_frame([crate::tokens::SIDE_PAD_X, y, row_w, label_h], viewport)
            {
                nodes.push(layout_node(
                    &format!("sidebar.group.{index}"),
                    "text",
                    frame,
                    [
                        text_width(label, crate::tokens::LABEL_SIZE),
                        crate::tokens::LABEL_SIZE,
                    ],
                    viewport,
                    true,
                    label,
                    true,
                ));
            }
            y += label_h;
            for row in placed.rows.iter().filter(|row| row.group == *label) {
                let raw = [crate::tokens::SIDE_PAD_X, y, row_w, crate::tokens::ROW_H];
                if let Some(frame) = clip_frame(raw, viewport) {
                    nodes.push(layout_node(
                        &format!("sidebar.row.{index}"),
                        "sidebar-row",
                        frame,
                        [row_w, crate::tokens::ROW_H],
                        viewport,
                        true,
                        &row.label,
                        true,
                    ));
                    if (frame[3] - crate::tokens::ROW_H).abs() < 0.5 {
                        push_row_pair(&mut nodes, &format!("sidebar.row.{index}"), frame, row);
                    }
                }
                y += crate::tokens::ROW_H;
                index += 1;
            }
        }
        if let Some(settings) = placed.rows.iter().find(|row| row.id == SelectId::Settings) {
            let frame = [
                crate::tokens::SIDE_PAD_X,
                settings_y,
                row_w,
                crate::tokens::ROW_H,
            ];
            nodes.push(layout_node(
                "sidebar.settings",
                "sidebar-row",
                frame,
                [row_w, crate::tokens::ROW_H],
                side_frame,
                true,
                &settings.label,
                false,
            ));
            push_row_pair(&mut nodes, "sidebar.settings", frame, settings);
        }
    }
    if let Some(chat) = placed.chat {
        nodes.push(layout_node(
            "chat",
            "container",
            [chat.x as f32, chat.y as f32, chat.w as f32, chat.h as f32],
            [chat.w as f32, chat.h as f32],
            window,
            false,
            "",
            false,
        ));
        push_chat_boxes(&mut nodes, chat, library);
    }
    let grid = spike_library::shelf_grid(content_w);
    let row_w = spike_library::row_width(grid);
    let header_y = rect.y as f32 + chrome.toolbar_h;
    let mut y = header_y + crate::tokens::CARD_PAD_TOP + crate::tokens::TITLE_ROW_H;
    let scroll_top = y;
    let scroll_bottom = rect.y as f32 + rect.h as f32 - crate::tokens::CARD_RADIUS as f32;
    if scroll_bottom > scroll_top {
        note_shelf_hit([origin_x, scroll_top, content_w, scroll_bottom - scroll_top]);
    }
    let continued =
        crate::tokens::continue_layout(content_w, library.continue_reading.len() as u32);
    let continue_section_y = y;
    if continued.cards > 0 {
        y += crate::tokens::section_block();
        let card_h = crate::tokens::continue_card_h();
        for index in 0..continued.cards.min(library.continue_reading.len() as u32) {
            let x = origin_x + index as f32 * (continued.card_w + continued.gap);
            let frame = [x, y, continued.card_w, card_h];
            let title = library
                .continue_reading
                .get(index as usize)
                .map(|card| card.title.as_str())
                .unwrap_or("");
            nodes.push(layout_node(
                &format!("continue.{index}"),
                "button",
                frame,
                [continued.card_w, card_h],
                content,
                false,
                title,
                false,
            ));
            nodes.push(layout_node(
                &format!("continue.{index}.cover"),
                "cover",
                [
                    x + crate::tokens::CONTINUE_PAD,
                    y + crate::tokens::CONTINUE_PAD,
                    continued.cover_w,
                    continued.cover_h,
                ],
                [continued.cover_w, continued.cover_h],
                frame,
                false,
                title,
                false,
            ));
            nodes.push(layout_node(
                &format!("continue.{index}.title"),
                "text",
                [
                    x + crate::tokens::CONTINUE_PAD
                        + continued.cover_w
                        + crate::tokens::CONTINUE_INNER_GAP,
                    y + crate::tokens::CONTINUE_PAD + crate::tokens::CONTINUE_TEXT_PT,
                    (continued.card_w
                        - crate::tokens::CONTINUE_PAD * 2.0
                        - continued.cover_w
                        - crate::tokens::CONTINUE_INNER_GAP)
                        .max(0.0),
                    crate::tokens::CONTINUE_TITLE_SIZE * 1.2,
                ],
                [
                    text_width(title, crate::tokens::CONTINUE_TITLE_SIZE),
                    crate::tokens::CONTINUE_TITLE_SIZE,
                ],
                frame,
                true,
                title,
                false,
            ));
        }
        y += card_h;
    }
    let shelf_section_y = y;
    y += crate::tokens::section_block();
    let row_h = grid.cover_h + grid.caption_h;
    nodes.push(layout_node(
        "shelf.row",
        "container",
        [origin_x, y, row_w, row_h],
        [row_w, row_h],
        content,
        false,
        "",
        false,
    ));
    for index in 0..grid.columns.min(library.covers.len() as u32) {
        let x = origin_x + index as f32 * (grid.cover_w + grid.column_gap);
        let frame = [x, y, grid.cover_w, grid.cover_h];
        let card = library.covers.get(index as usize);
        let title = card.map(|item| item.title.as_str()).unwrap_or("");
        let mut cover = layout_node(
            &format!("cover.{index}"),
            "cover",
            frame,
            [grid.cover_w, grid.cover_h],
            content,
            false,
            title,
            false,
        );
        if card.is_some_and(coverless) {
            let color = card
                .map(painted_plate_color)
                .unwrap_or_else(|| fallback_color(title));
            cover["plate"] = serde_json::json!(true);
            cover["plate_rgb"] = serde_json::json!(format!("{:06X}", color & 0x00FF_FFFF));
            nodes.push(cover);
            let (title_size, author_size) = plate_type_size(grid.cover_h);
            let title_h = plate_line_box(title_size);
            let author_h = plate_line_box(author_size);
            let text_w = (grid.cover_w - crate::tokens::COVER_LABEL_PX * 2.0).max(0.0);
            let block = title_h + crate::tokens::SHELF_META_GAP + author_h;
            let text_x = x + crate::tokens::COVER_LABEL_PX;
            let text_y = y + (grid.cover_h - block) / 2.0;
            let author = card.map(|item| item.author.as_str()).unwrap_or("");
            nodes.push(layout_node(
                &format!("cover.{index}.type.title"),
                "text",
                [text_x, text_y, text_w, title_h],
                [text_width(title, title_size), title_h],
                frame,
                true,
                title,
                false,
            ));
            nodes.push(layout_node(
                &format!("cover.{index}.type.author"),
                "text",
                [
                    text_x,
                    text_y + title_h + crate::tokens::SHELF_META_GAP,
                    text_w,
                    author_h,
                ],
                [text_width(author, author_size), author_h],
                frame,
                true,
                author,
                false,
            ));
        } else {
            nodes.push(cover);
        }
        nodes.push(layout_node(
            &format!("cover.{index}.title"),
            "text",
            [x, y + grid.cover_h, grid.cover_w, grid.caption_h],
            [
                text_width(title, crate::tokens::SHELF_TITLE_SIZE),
                crate::tokens::SHELF_TITLE_SIZE,
            ],
            [x, y + grid.cover_h, grid.cover_w, grid.caption_h],
            true,
            title,
            true,
        ));
    }
    push_chrome_boxes(
        &mut nodes,
        placed,
        rect,
        content,
        origin_x,
        content_w,
        header_y,
        if continued.cards > 0 {
            Some(continue_section_y)
        } else {
            None
        },
        shelf_section_y,
        library.book_count,
    );
    nodes
}

fn row_icon_size(row: &SideRow) -> f32 {
    if row.dot.is_some() {
        crate::tokens::DOT
    } else if row.badge.is_some() {
        crate::tokens::SOURCE_BADGE
    } else if row.icon == "plus" {
        crate::tokens::ADD_ICON
    } else if row.id == SelectId::Settings {
        crate::tokens::SETTINGS_ICON
    } else {
        crate::tokens::ROW_ICON
    }
}

fn push_row_pair(nodes: &mut Vec<serde_json::Value>, id: &str, frame: [f32; 4], row: &SideRow) {
    let icon = row_icon_size(row);
    let icon_frame = [
        frame[0] + crate::tokens::ROW_PAD_L + (crate::tokens::ROW_ICON_SLOT - icon) / 2.0,
        frame[1] + (crate::tokens::ROW_H - icon) / 2.0,
        icon,
        icon,
    ];
    nodes.push(layout_node(
        &format!("{id}.icon"),
        "icon",
        icon_frame,
        [icon, icon],
        frame,
        false,
        row.icon,
        true,
    ));
    let line = crate::tokens::BODY_SIZE * 1.2;
    let label_x =
        frame[0] + crate::tokens::ROW_PAD_L + crate::tokens::ROW_ICON_SLOT + crate::tokens::ROW_GAP;
    let label_w = (frame[2]
        - crate::tokens::ROW_PAD_L
        - crate::tokens::ROW_ICON_SLOT
        - crate::tokens::ROW_GAP
        - crate::tokens::ROW_PAD_R
        - 24.0)
        .max(0.0);
    let label_frame = [
        label_x,
        frame[1] + (crate::tokens::ROW_H - line) / 2.0,
        label_w,
        line,
    ];
    nodes.push(layout_node(
        &format!("{id}.label"),
        "text",
        label_frame,
        [text_width(&row.label, crate::tokens::BODY_SIZE), line],
        frame,
        true,
        &row.label,
        true,
    ));
}

fn push_chat_boxes(nodes: &mut Vec<serde_json::Value>, chat: Rect, library: &LibraryIn) {
    let chat_box = [chat.x as f32, chat.y as f32, chat.w as f32, chat.h as f32];
    let head_h = crate::tokens::CHAT_HEAD_H;
    let head = [chat.x as f32, chat.y as f32, chat.w as f32, head_h];
    nodes.push(layout_node(
        "chat.head",
        "container",
        head,
        [chat.w as f32, head_h],
        chat_box,
        false,
        "",
        false,
    ));
    let avatar = crate::tokens::AVATAR;
    let row_top = chat.y as f32;
    nodes.push(layout_node(
        "chat.avatar",
        "chrome",
        [
            chat.x as f32 + crate::tokens::CHAT_PAD_X,
            row_top + crate::tokens::chrome_top(avatar),
            avatar,
            avatar,
        ],
        [avatar, avatar],
        head,
        false,
        "慢",
        false,
    ));
    let name_line = crate::tokens::NAME_SIZE * 1.2;
    let name_y = row_top + crate::tokens::chrome_top(name_line);
    let name_x = chat.x as f32 + crate::tokens::CHAT_PAD_X + avatar + crate::tokens::CHAT_HEAD_GAP;
    nodes.push(layout_node(
        "chat.name",
        "text",
        [
            name_x,
            name_y,
            text_width("Slowpage", crate::tokens::NAME_SIZE),
            name_line,
        ],
        [text_width("Slowpage", crate::tokens::NAME_SIZE), name_line],
        head,
        true,
        "Slowpage",
        false,
    ));
    let sub_line = crate::tokens::SUB_SIZE * 1.2;
    nodes.push(layout_node(
        "chat.sub",
        "text",
        [
            name_x + text_width("Slowpage", crate::tokens::NAME_SIZE) + crate::tokens::NAME_GAP,
            name_y + name_line - sub_line,
            text_width("你的阅读助手", crate::tokens::SUB_SIZE),
            sub_line,
        ],
        [
            text_width("你的阅读助手", crate::tokens::SUB_SIZE),
            sub_line,
        ],
        head,
        true,
        "你的阅读助手",
        false,
    ));
    let button = crate::tokens::TOGGLE_SIZE;
    let cluster = button * 2.0 + crate::tokens::TOOL_CLUSTER_GAP;
    let cluster_x = chat.x as f32 + chat.w as f32 - crate::tokens::CHAT_PAD_X - cluster;
    let button_y = row_top + crate::tokens::chrome_top(button);
    nodes.push(layout_node(
        "chat.new",
        "chrome",
        [cluster_x, button_y, button, button],
        [button, button],
        head,
        false,
        "新对话",
        false,
    ));
    nodes.push(layout_node(
        "chat.panel",
        "chrome",
        [
            cluster_x + button + crate::tokens::TOOL_CLUSTER_GAP,
            button_y,
            button,
            button,
        ],
        [button, button],
        head,
        false,
        "对话",
        false,
    ));
    push_composer(nodes, chat, library);
}

fn scope_chip_width(title: &str) -> f32 {
    crate::tokens::SCOPE_PX * 2.0
        + crate::tokens::SOURCE_GLYPH
        + crate::tokens::SCOPE_GAP
        + text_width(title, crate::tokens::SUB_SIZE)
}

fn chip_limit(chat_w: f32) -> f32 {
    let inner = chat_w - crate::tokens::COMPOSER_INSET * 2.0 - crate::tokens::COMPOSER_PAD * 2.0;
    (inner
        - crate::tokens::ATTACH_ICON
        - crate::tokens::COMPOSER_ROW_GAP
        - crate::tokens::COMPOSER_CONTROL
        - crate::tokens::COMPOSER_ROW_GAP)
        .max(0.0)
}

fn push_composer(nodes: &mut Vec<serde_json::Value>, chat: Rect, library: &LibraryIn) {
    let inset = crate::tokens::COMPOSER_INSET;
    let pad = crate::tokens::COMPOSER_PAD;
    let control = crate::tokens::COMPOSER_CONTROL;
    let line = crate::tokens::BODY_SIZE * 1.2;
    let composer_h = pad + line + crate::tokens::COMPOSER_ROW_MT + control + pad;
    let composer_w = (chat.w as f32 - inset * 2.0).max(0.0);
    let composer_x = chat.x as f32 + inset;
    let composer_y = chat.y as f32 + chat.h as f32 - inset - composer_h;
    let composer = [composer_x, composer_y, composer_w, composer_h];
    let chat_box = [chat.x as f32, chat.y as f32, chat.w as f32, chat.h as f32];
    nodes.push(layout_node(
        "composer",
        "container",
        composer,
        [composer_w, composer_h],
        chat_box,
        false,
        "问问 Slowpage…",
        false,
    ));
    let placeholder = "问问 Slowpage…";
    nodes.push(layout_node(
        "composer.placeholder",
        "text",
        [
            composer_x + pad,
            composer_y + pad,
            text_width(placeholder, crate::tokens::BODY_SIZE),
            line,
        ],
        [text_width(placeholder, crate::tokens::BODY_SIZE), line],
        composer,
        true,
        placeholder,
        false,
    ));
    let row_y = composer_y + pad + line + crate::tokens::COMPOSER_ROW_MT;
    nodes.push(layout_node(
        "composer.attach",
        "chrome",
        [composer_x + pad, row_y, crate::tokens::ATTACH_ICON, control],
        [crate::tokens::ATTACH_ICON, control],
        composer,
        false,
        "附件",
        false,
    ));
    let book = library
        .continue_reading
        .first()
        .map(|card| card.title.as_str())
        .unwrap_or("");
    let send_x = composer_x + composer_w - pad - control;
    if !book.is_empty() {
        let natural = scope_chip_width(book);
        let width = natural.min(chip_limit(chat.w as f32));
        nodes.push(layout_node(
            "composer.chip",
            "button",
            [
                composer_x + pad + crate::tokens::ATTACH_ICON + crate::tokens::COMPOSER_ROW_GAP,
                row_y,
                width,
                control,
            ],
            [natural, control],
            composer,
            natural > width,
            book,
            false,
        ));
    }
    nodes.push(layout_node(
        "composer.send",
        "chrome",
        [send_x, row_y, control, control],
        [control, control],
        composer,
        false,
        "发送",
        false,
    ));
}

fn push_chrome_boxes(
    nodes: &mut Vec<serde_json::Value>,
    placed: &Placement,
    rect: Rect,
    content: [f32; 4],
    origin_x: f32,
    content_w: f32,
    header_y: f32,
    continue_section_y: Option<f32>,
    shelf_section_y: f32,
    book_count: u32,
) {
    let card = [rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32];
    let chrome = crate::tokens::chrome_row(placed.chrome);
    let button = crate::tokens::TOGGLE_SIZE;
    if let Some(side) = placed.sidebar {
        nodes.push(layout_node(
            "sidebar.toggle",
            "chrome",
            [
                side.x as f32 + chrome.toggle_x,
                side.y as f32 + chrome.toggle_y,
                button,
                button,
            ],
            [button, button],
            [side.x as f32, side.y as f32, side.w as f32, side.h as f32],
            false,
            "侧边栏",
            false,
        ));
    } else if !matches!(placed.face, CardFace::Page) {
        nodes.push(layout_node(
            "sidebar.toggle",
            "chrome",
            [chrome.toggle_x, chrome.toggle_y, button, button],
            [button, button],
            card,
            false,
            "侧边栏",
            false,
        ));
    }
    let toolbar_h = chrome.toolbar_h;
    let toolbar = [rect.x as f32, rect.y as f32, rect.w as f32, toolbar_h];
    nodes.push(layout_node(
        "toolbar",
        "container",
        toolbar,
        [rect.w as f32, toolbar_h],
        card,
        false,
        "",
        false,
    ));
    let sort_w = crate::tokens::TOOL_PX * 2.0
        + crate::tokens::SORT_ICON
        + crate::tokens::TOOL_GAP
        + text_width("最近阅读", crate::tokens::TOOL_SIZE)
        + crate::tokens::TOOL_GAP
        + crate::tokens::CHEVRON_ICON;
    let view_w = crate::tokens::SEG_PAD * 2.0 + crate::tokens::SEG_BTN_W * 2.0;
    let view_h = crate::tokens::SEG_BTN_H + crate::tokens::SEG_PAD * 2.0;
    let view_x = rect.x as f32 + rect.w as f32 - crate::tokens::TOOLBAR_PX - view_w;
    let sort_x = view_x - crate::tokens::TOOLBAR_GAP - sort_w;
    nodes.push(layout_node(
        "toolbar.sort",
        "chrome",
        [
            sort_x,
            rect.y as f32 + (toolbar_h - crate::tokens::TOOL_H) / 2.0,
            sort_w,
            crate::tokens::TOOL_H,
        ],
        [sort_w, crate::tokens::TOOL_H],
        toolbar,
        false,
        "最近阅读",
        false,
    ));
    nodes.push(layout_node(
        "toolbar.view",
        "chrome",
        [
            view_x,
            rect.y as f32 + (toolbar_h - view_h) / 2.0,
            view_w,
            view_h,
        ],
        [view_w, view_h],
        toolbar,
        false,
        "网格",
        false,
    ));
    let header = [
        origin_x,
        header_y + crate::tokens::CARD_PAD_TOP,
        content_w,
        crate::tokens::TITLE_ROW_H,
    ];
    nodes.push(layout_node(
        "shelf.header",
        "container",
        header,
        [content_w, crate::tokens::TITLE_ROW_H],
        content,
        false,
        "",
        false,
    ));
    let selected = placed
        .rows
        .iter()
        .find(|row| row.selected)
        .map(|row| row.id.clone())
        .unwrap_or(SelectId::AllBooks);
    let title = selected_title(&selected);
    let title_h = crate::tokens::H1_SIZE;
    let title_y = header[1] + (crate::tokens::TITLE_ROW_H - title_h) / 2.0;
    nodes.push(layout_node(
        "shelf.header.title",
        "text",
        [origin_x, title_y, content_w * 0.7, title_h],
        [text_width(&title, title_h), title_h],
        header,
        true,
        &title,
        false,
    ));
    let count = format!("{book_count} 本");
    let count_h = crate::tokens::COUNT_SIZE;
    let count_w = text_width(&count, count_h);
    nodes.push(layout_node(
        "shelf.header.count",
        "text",
        [
            origin_x + content_w - count_w,
            title_y + title_h - count_h,
            count_w,
            count_h,
        ],
        [count_w, count_h],
        header,
        false,
        &count,
        false,
    ));
    if let Some(section_y) = continue_section_y {
        push_section(
            nodes,
            "continue.section",
            "继续阅读",
            origin_x,
            content_w,
            section_y,
            content,
        );
    }
    push_section(
        nodes,
        "shelf.section",
        "书架",
        origin_x,
        content_w,
        shelf_section_y,
        content,
    );
}

fn push_section(
    nodes: &mut Vec<serde_json::Value>,
    id: &str,
    label: &str,
    x: f32,
    width: f32,
    y: f32,
    content: [f32; 4],
) {
    let line = crate::tokens::SECTION_SIZE * 1.2;
    nodes.push(layout_node(
        id,
        "text",
        [x, y + crate::tokens::SECTION_MT, width, line],
        [text_width(label, crate::tokens::SECTION_SIZE), line],
        content,
        true,
        label,
        false,
    ));
}

fn traffic_nodes(window_w: f32, window_h: f32) -> Vec<serde_json::Value> {
    let Ok(text) = std::fs::read_to_string("/tmp/superreader-spike/chrome.json") else {
        return Vec::new();
    };
    let Ok(body) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let height = body
        .get("window_h")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(window_h);
    let chrome_w = body
        .get("window_w")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(window_w);
    if (chrome_w - window_w).abs() > 1.0 || (height - window_h).abs() > 1.0 {
        return Vec::new();
    }
    // Hidden buttons are not on screen. Their frames may still sit in the
    // title-bar band after a hover, and exporting them would overlap the toggle.
    if body
        .get("lights_visible")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Vec::new();
    }
    let mut nodes = Vec::new();
    for (id, key) in [
        ("traffic.close", "close"),
        ("traffic.miniaturize", "miniaturize"),
        ("traffic.zoom", "zoom"),
    ] {
        let Some(frame) = body.get(key) else {
            continue;
        };
        let Some(node) = light_node(id, frame, window_w, height) else {
            continue;
        };
        nodes.push(node);
    }
    nodes
}

fn light_node(
    id: &str,
    frame: &serde_json::Value,
    window_w: f32,
    window_h: f32,
) -> Option<serde_json::Value> {
    let x = frame.get("x")?.as_f64()? as f32;
    let y_up = frame.get("y")?.as_f64()? as f32;
    let w = frame.get("w")?.as_f64()? as f32;
    let h = frame.get("h")?.as_f64()? as f32;
    if w < 1.0 || h < 1.0 {
        return None;
    }
    let y = window_h - (y_up + h);
    Some(layout_node(
        id,
        "traffic",
        [x, y, w, h],
        [w, h],
        [0.0, 0.0, window_w, window_h],
        false,
        "",
        false,
    ))
}

fn clip_frame(frame: [f32; 4], view: [f32; 4]) -> Option<[f32; 4]> {
    let top = frame[1].max(view[1]);
    let bottom = (frame[1] + frame[3]).min(view[1] + view[3]);
    let height = bottom - top;
    if height < 1.0 {
        None
    } else {
        Some([frame[0], top, frame[2], height])
    }
}

fn coverless(card: &CoverCard) -> bool {
    !(card.cover.is_some() && card.plate_rgb.is_none() && !card.pdf)
}

fn plate_type_size(height: f32) -> (f32, f32) {
    if height < 100.0 {
        (crate::tokens::TAG_SM_SIZE, crate::tokens::TAG_SM_SIZE)
    } else {
        (crate::tokens::SECTION_SIZE, crate::tokens::TAG_SIZE)
    }
}

/// NSH's line box is 1.2 of the font size when line height is unset. The
/// cover clips its children, so the frame has to hold that box.
fn plate_line_box(size: f32) -> f32 {
    (size * 1.2).ceil()
}

fn write_frames(placed: &Placement, library: &LibraryIn) {
    let CardBox::Placed { rect, .. } = placed.card else {
        return;
    };
    let window_w = if let Some(chat) = placed.chat {
        chat.x + chat.w
    } else {
        rect.x + rect.w + INSET
    };
    let window_h = rect.y + rect.h + INSET;
    let mut nodes = layout_nodes(placed, library);
    nodes.extend(traffic_nodes(window_w as f32, window_h as f32));
    let body = serde_json::json!({
        "window": [window_w, window_h],
        "chrome_center": crate::tokens::chrome_center(),
        "inset": INSET,
        "sidebar_pad_x": crate::tokens::SIDE_PAD_X,
        "card_pad_x": crate::tokens::CARD_PAD_X,
        "chat_pad_x": crate::tokens::CHAT_PAD_X,
        "window_corner": crate::tokens::WINDOW_CORNER,
        "composer_inset": crate::tokens::COMPOSER_INSET,
        "composer_pad": crate::tokens::COMPOSER_PAD,
        "lights": placed.chrome.lights && placed.chrome.bar > 8.0,
        "traffic_bar": placed.chrome.bar,
        "content_top": placed.chrome.top,
        "pid": std::process::id(),
        "nodes": nodes,
    });
    crate::stage::trace_resize("layout-json", window_w, window_h, true);
    let path = std::path::Path::new("/tmp/superreader-spike/layout.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, body.to_string());
}

fn paint(
    placed: Placement,
    form: Signal<Form>,
    selected: Signal<SelectId>,
    menu: Signal<bool>,
    query: Signal<String>,
    library: LibraryIn,
    on_peer: impl Fn(Peer) + Clone + 'static,
    on_open: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    let mut root = view().absolute().inset(px(0.0));
    if placed.gray {
        root = root
            .bg_linear_to_b()
            .gradient_from(rgb(crate::tokens::FIELD_TOP).to_hsla())
            .gradient_to(rgb(crate::tokens::FIELD_BOTTOM).to_hsla());
    }
    if matches!(form.get(), Form::Full { .. }) {
        fonts_clear();
    }
    if let CardBox::Placed {
        rect,
        radius,
        shadow,
    } = placed.card
    {
        if shadow {
            root = root.child(
                view()
                    .absolute()
                    .left(px(rect.x as f32))
                    .top(px(rect.y as f32))
                    .w(px(rect.w as f32))
                    .h(px(rect.h as f32))
                    .rounded(px(radius as f32))
                    .bg(rgb(crate::tokens::CARD).to_hsla())
                    .shadow(crate::tokens::card_shadows()),
            );
        }
    }
    if let Some(rect) = placed.sidebar {
        root = root.child(sidebar_pane(
            Some(rect),
            &placed,
            form,
            selected,
            menu,
            query.clone(),
            on_peer.clone(),
        ));
    }
    if let Some(rect) = placed.chat {
        root = root.child(chat_pane(Some(rect), form, &library));
    }
    if let CardBox::Placed { rect, radius, .. } = placed.card {
        root = root.child(card_face(
            Some(rect),
            radius,
            placed.face,
            &library,
            on_peer,
            on_open,
        ));
    }
    if matches!(form.get(), Form::Full { .. }) {
        fonts_write();
    }
    if placed.sidebar.is_none() && !matches!(form.get(), Form::Immersive) {
        root = root.child(sidebar_toggle_button(
            crate::tokens::chrome_row(placed.chrome),
            form,
        ));
    }
    if placed.chat.is_none() && matches!(form.get(), Form::Full { .. }) {
        root = root.child(panel_toggle_button(
            form,
            true,
            crate::tokens::chrome_row(placed.chrome),
        ));
    }
    root.into_element()
}

fn sidebar_pane(
    rect: Option<Rect>,
    placed: &Placement,
    form: Signal<Form>,
    selected: Signal<SelectId>,
    menu: Signal<bool>,
    query: Signal<String>,
    on_peer: impl Fn(Peer) + Clone + 'static,
) -> AnyElement {
    let mut column = view().w(px(SIDEBAR_W as f32)).h_full().flex().flex_col();
    if let Some(rect) = rect {
        column = column
            .absolute()
            .left(px(rect.x as f32))
            .top(px(rect.y as f32))
            .w(px(rect.w as f32))
            .h(px(rect.h as f32));
    }
    column = column.child(sidebar_top(crate::tokens::chrome_row(placed.chrome), form));
    column = column.child(
        view()
            .px(px(crate::tokens::SIDE_PAD_X))
            .child(search_field(query)),
    );
    let mut groups = scroll()
        .flex_1()
        .flex_shrink(1.0)
        .min_h(px(0.0))
        .overflow_hidden()
        .w_full()
        .flex()
        .flex_col()
        .scrollable(true)
        .px(px(crate::tokens::SIDE_PAD_X))
        .pb(px(crate::tokens::SIDE_PAD_BOTTOM));
    for label in &placed.groups {
        groups = groups.child(group_label(label));
        for row in placed.rows.iter().filter(|row| row.group == *label) {
            groups = groups.child(side_row(row.clone(), selected, menu));
        }
    }
    column = column.child(groups);
    if let Some(settings) = placed.rows.iter().find(|row| row.id == SelectId::Settings) {
        column = column.child(
            view()
                .px(px(crate::tokens::SIDE_PAD_X))
                .pb(px(crate::tokens::SIDE_PAD_BOTTOM))
                .child(side_row(settings.clone(), selected, menu)),
        );
    }
    if menu.get() {
        column = column.child(source_menu(menu, on_peer));
    }
    column.into_element()
}

fn source_menu(menu: Signal<bool>, on_peer: impl Fn(Peer) + Clone + 'static) -> AnyElement {
    let choices = [
        ("Apple Books", Peer::AppleBooks),
        ("calibre", Peer::Calibre),
        ("添加文件夹", Peer::AddFolder),
    ];
    let mut list = view()
        .absolute()
        .left(px(crate::tokens::SIDE_PAD_X))
        .bottom(px(crate::tokens::SIDE_PAD_BOTTOM + crate::tokens::ROW_H))
        .w(px(188.0))
        .rounded(px(12.0))
        .bg(rgb(0x00FF_FFFF).to_hsla())
        .shadow_lg()
        .p(px(6.0))
        .flex()
        .flex_col();
    for (label, peer) in choices {
        let menu = menu.clone();
        let on_peer = on_peer.clone();
        list = list.child(
            view()
                .h(px(32.0))
                .rounded(px(8.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .accessible(true)
                .label(label)
                .on_press(move || {
                    menu.set(false);
                    on_peer(peer);
                })
                .child(
                    text(label.to_string())
                        .text_size(px(13.0))
                        .text_color(ink()),
                ),
        );
    }
    list.into_element()
}

fn sidebar_top(row: crate::tokens::ChromeRow, form: Signal<Form>) -> AnyElement {
    view()
        .h(px(row.height))
        .w_full()
        .child(sidebar_toggle_button(row, form))
        .into_element()
}

fn sidebar_toggle_button(row: crate::tokens::ChromeRow, form: Signal<Form>) -> AnyElement {
    let mut button = view()
        .absolute()
        .left(px(row.toggle_x))
        .top(px(row.toggle_y))
        .w(px(crate::tokens::TOGGLE_SIZE))
        .h(px(crate::tokens::TOGGLE_SIZE))
        .rounded(px(crate::tokens::TOGGLE_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("侧边栏")
        .on_press(move || {
            let next = step(form.get(), Command::ToggleSidebar);
            form.set(next);
        });
    if let Some(path) = crate::icons::glyph_ink("sidebar-left", "6E6E73") {
        button = button.child(image(path).size(px(crate::tokens::SIDEBAR_ICON)));
    }
    button.into_element()
}

/// A floating toggle is on screen only because the panel is not. When the
/// preference is already Shown, the panel is hidden by the window width, and
/// a click must not record Hidden.
fn panel_command(current: Form, floating: bool) -> Form {
    match current {
        Form::Full {
            sidebar,
            chat: Presence::Shown,
        } if floating => Form::Full {
            sidebar,
            chat: Presence::Shown,
        },
        _ => step(current, Command::TogglePanel),
    }
}

thread_local! {
    static FIND_STEP: RefCell<
        Option<(
            Signal<spike_library::column::Column>,
            Signal<spike_library::margin::LivePage>,
            ElementRef,
        )>,
    > = const { RefCell::new(None) };
}

/// Shift+Return, ⌘G, and Shift-⌘G step through find. They exist only while
/// find is open and the field is not composing. A closed find and an open
/// composition do not register them, so the key still reaches the field.
#[component]
fn FindNavigationKeys() -> impl IntoElement {
    if let Some((column, live, stage)) = FIND_STEP.with(|slot| slot.borrow().clone()) {
        let current = column.get();
        if spike_library::column::find_navigation_mounted(current.mode, current.find.composing) {
            let options = KeyOptions {
                when: nsh_core::keymap::When::Always,
                pass_through: false,
                ..KeyOptions::default()
            };
            on_key_with("shift+return", options, {
                let column = column.clone();
                let live = live.clone();
                let stage = stage.clone();
                move |_| press_find_step(&column, &live, &stage, false)
            });
            on_key_with("mod+g", options, {
                let column = column.clone();
                let live = live.clone();
                let stage = stage.clone();
                move |_| press_find_step(&column, &live, &stage, true)
            });
            on_key_with("shift+mod+g", options, {
                let column = column.clone();
                let live = live.clone();
                let stage = stage.clone();
                move |_| press_find_step(&column, &live, &stage, false)
            });
        }
    }
    view().absolute().w(px(0.0)).h(px(0.0))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn reading_place(
    live: &spike_library::margin::LivePage,
) -> spike_library::column::Place {
    if live.spine.is_empty() {
        return spike_library::column::Place {
            href: String::new(),
            page: live.page,
            offset: 0,
        };
    }
    let place = crate::web::reader_place();
    spike_library::column::Place {
        href: if place.href.is_empty() {
            live.spine.clone()
        } else {
            place.href
        },
        page: if place.page == 0 {
            live.page
        } else {
            place.page
        },
        offset: place.offset,
    }
}

fn load_marks(column: &spike_library::column::Column) -> spike_library::column::Column {
    let path = crate::settings::support_dir().join("annotations.sqlite");
    match spike_library::column::Marks::open(&path) {
        Ok(marks) => {
            let rows = marks.list(&column.book).unwrap_or_default();
            spike_library::column::reload(column, rows)
        }
        Err(_) => column.clone(),
    }
}

fn send_find(stage: &ElementRef, column: &spike_library::column::Column) {
    stage.command(
        "find",
        serde_json::json!({
            "book": column.book,
            "query": column.find.query,
            "generation": column.find.generation,
            "step": column.find.pending,
        }),
    );
}

pub(crate) fn press_find_step(
    column: &Signal<spike_library::column::Column>,
    live: &Signal<spike_library::margin::LivePage>,
    stage: &ElementRef,
    forward: bool,
) {
    if column.get().mode != spike_library::column::Mode::Find {
        return;
    }
    let place = reading_place(&live.get());
    let (next, hit) = spike_library::column::step(&column.get(), forward, &place);
    if let Some(hit) = hit.as_ref() {
        send_jump(stage, hit);
    }
    column.set(next);
}

fn send_jump(stage: &ElementRef, hit: &spike_library::column::Hit) {
    stage.command(
        "jump",
        serde_json::json!({
            "href": hit.href,
            "page": hit.page,
            "offset": hit.start,
            "end": hit.end,
        }),
    );
}

pub fn apply_find_status(
    column: &spike_library::column::Column,
    value: &serde_json::Value,
    here: &spike_library::column::Place,
) -> (
    spike_library::column::Column,
    Option<spike_library::column::Hit>,
) {
    let Some(find) = value.get("find") else {
        return (column.clone(), None);
    };
    if find.is_null() {
        return (column.clone(), None);
    }
    let kind = |name: &str| {
        if name == "chars" {
            spike_library::column::IndexKind::Chars
        } else {
            spike_library::column::IndexKind::Utf16
        }
    };
    let hits = find
        .get("hits")
        .and_then(|hits| hits.as_array())
        .map(|hits| {
            hits.iter()
                .filter_map(|hit| {
                    Some(spike_library::column::Hit {
                        page: hit.get("page")?.as_u64()? as u32,
                        href: hit.get("href")?.as_str()?.to_string(),
                        start: hit.get("start")?.as_u64()? as u32,
                        end: hit.get("end")?.as_u64()? as u32,
                        kind: kind(hit.get("kind").and_then(|k| k.as_str()).unwrap_or("utf16")),
                        snippet: hit.get("snippet")?.as_str()?.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let scan = spike_library::column::Scan {
        book: find
            .get("book")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        query: find
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        generation: find.get("generation").and_then(|v| v.as_u64()).unwrap_or(0),
        hits,
        seen: find.get("seen").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        total: find.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        text_units: find.get("text_units").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        empty_units: find
            .get("empty_units")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        failed_units: find
            .get("failed_units")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        done: find.get("done").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    spike_library::column::replace_scan(column, &scan, here)
}

pub fn reading_margin(
    form: Signal<Form>,
    window: Signal<Window>,
    chrome: Signal<crate::tokens::ChromeIn>,
    stage: ElementRef,
    ease: Signal<u64>,
    live: Signal<spike_library::margin::LivePage>,
    hold: Signal<spike_library::margin::Hold>,
    fault: Signal<String>,
    note: Signal<spike_library::margin::MarginInk>,
    book_gen: Signal<u64>,
    measures: Signal<Vec<spike_library::margin::FreshMeasure>>,
    column: Signal<spike_library::column::Column>,
) -> AnyElement {
    let _tick = ease.get();
    let _form = form.get();
    let _note = note.get();
    let size = window.get();
    let row = chrome.get();
    let top = f64::from(row.top.max(0.0));
    let trip = crate::ease::current();
    let frame = crate::ease::present(size.width, size.height, top, trip);
    let expired = spike_library::column::expire_undo(&column.get(), now_ms());
    if expired != column.get() {
        column.set(expired);
    }
    let mode = column.get().mode;
    let column_rect = frame.margin.or_else(|| {
        if mode == spike_library::column::Mode::Notes || !immersive_on() {
            None
        } else {
            Some(Rect {
                x: (size.width - MARGIN_W).max(0.0),
                y: 0.0,
                w: MARGIN_W.min(size.width.max(0.0)),
                h: size.height.max(1.0),
            })
        }
    });
    let live_now = live.get();
    let mut live_for_page = live_now.clone();
    live_for_page.moving = live_for_page.moving || trip.moving();
    let page_hold = spike_library::margin::reconcile(&hold.get(), &live_for_page);
    let loaded = if page_hold.spine.is_empty() {
        load_margin(&page_hold.book, page_hold.page, "")
    } else {
        load_spine(&page_hold.book, &page_hold.spine)
    };
    let (notes, error) = match loaded {
        Ok(notes) if page_hold.spine.is_empty() => (notes, String::new()),
        Ok(notes) => (
            spike_library::margin::epub_shown(&notes, &measures.get(), page_hold.page),
            String::new(),
        ),
        Err(err) => (Vec::new(), err),
    };
    if fault.get() != error {
        fault.set(error);
    }
    let margin_x = frame.margin.map(|rect| rect.x).unwrap_or(0.0);
    let ink_now = note.get();
    let editor = card_editor_height(&ink_now, &notes, &page_hold.book);
    let settled = settle_margin(
        &hold.get(),
        &live_now,
        trip.moving(),
        &frame.page,
        margin_x,
        size.height,
        &notes,
        editor,
    );
    if hold.get() != settled.hold {
        hold.set(settled.hold);
    }
    let scene = settled.scene;
    let mut root = view()
        .absolute()
        .size_full()
        .pointer_events(PointerEvents::BoxNone);
    if let Some(rect) = column_rect {
        root = root.child(
            view()
                .absolute()
                .left(px(rect.x as f32))
                .top(px(rect.y as f32))
                .w(px(rect.w as f32))
                .h(px(rect.h as f32))
                .bg(rgb(FIELD_HEX).to_hsla()),
        );
        root = root.child(column_tabs(
            rect,
            column.clone(),
            note.clone(),
            live.clone(),
            stage.clone(),
        ));
        for placed in &scene.cards {
            for seg in &placed.segments {
                if seg.w < 0.5 && seg.h < 0.5 {
                    continue;
                }
                let alpha = if seg.fade {
                    spike_library::margin::CONNECTOR_FADE as f32
                } else {
                    1.0
                };
                root = root.child(
                    view()
                        .absolute()
                        .left(px(seg.x as f32))
                        .top(px(seg.y as f32))
                        .w(px(seg.w.max(1.0) as f32))
                        .h(px(seg.h.max(1.0) as f32))
                        .bg(ink())
                        .opacity(alpha)
                        .pointer_events(PointerEvents::BoxNone),
                );
            }
        }
        let band = spike_library::margin::band_bottom(size.height);
        let clip_h = (band - spike_library::margin::BAND_TOP).max(0.0);
        let mut clip = view()
            .absolute()
            .left(px(rect.x as f32))
            .top(px(spike_library::margin::BAND_TOP as f32))
            .w(px(rect.w as f32))
            .h(px(clip_h as f32))
            .overflow_hidden()
            .pointer_events(PointerEvents::Auto)
            .on_wheel({
                let hold = hold.clone();
                move |event: &WheelEvent| {
                    let dy = f64::from(event.delta.points(Points(16.0)).y.0);
                    let mut current = hold.get();
                    let scrolled =
                        (current.scroll + dy).clamp(current.scroll_min, current.scroll_max);
                    if (scrolled - current.scroll).abs() > 0.01 {
                        current.scroll = scrolled;
                        hold.set(current);
                    }
                }
            });
        let ink_signal = note.clone();
        for placed in &scene.cards {
            if let Some(stored) = notes.iter().find(|stored| stored.id == placed.id) {
                clip = clip.child(margin_card(
                    stored,
                    placed,
                    hold.clone(),
                    ink_signal.clone(),
                    &page_hold.book,
                ));
            }
        }
        if mode == spike_library::column::Mode::Notes {
            root = root.child(clip);
            let composer_top = band;
            root = root.child(margin_composer(
                note.clone(),
                live.clone(),
                book_gen.clone(),
                rect,
                composer_top,
            ));
        } else {
            root = root.child(column_body(
                column.clone(),
                live.clone(),
                stage.clone(),
                rect,
                size.height,
            ));
        }
    }
    if frame.panel_toggle {
        root = root.child(margin_toggle(
            form.clone(),
            row,
            stage.clone(),
            note.clone(),
        ));
    }
    root = root.child(immersive_button(
        form,
        crate::ease::button_seat(size.width, top),
        frame.plate,
        stage,
    ));
    root.into_element()
}

fn column_tabs(
    rect: Rect,
    column: Signal<spike_library::column::Column>,
    note: Signal<spike_library::margin::MarginInk>,
    live: Signal<spike_library::margin::LivePage>,
    stage: ElementRef,
) -> AnyElement {
    let mode = column.get().mode;
    let mut row = view()
        .absolute()
        .left(px(rect.x as f32 + 12.0))
        .top(px(rect.y as f32 + 8.0))
        .h(px(28.0))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0));
    for (label, next_mode) in [
        ("批注", spike_library::column::Mode::Notes),
        ("书签", spike_library::column::Mode::Marks),
        ("搜索", spike_library::column::Mode::Find),
    ] {
        let column = column.clone();
        let note = note.clone();
        let live = live.clone();
        let stage = stage.clone();
        let active = mode == next_mode;
        row = row.child(
            view()
                .on_press(move || press_column_mode(next_mode, &column, &note, &live, &stage))
                .child(
                    line!(
                        label,
                        crate::tokens::BODY_SIZE,
                        if active {
                            crate::tokens::NAME_WEIGHT
                        } else {
                            crate::tokens::BODY_WEIGHT
                        },
                        if active { ink() } else { quiet() },
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                ),
        );
    }
    row.into_element()
}

fn press_column_mode(
    mode: spike_library::column::Mode,
    column: &Signal<spike_library::column::Column>,
    note: &Signal<spike_library::margin::MarginInk>,
    live: &Signal<spike_library::margin::LivePage>,
    stage: &ElementRef,
) {
    let current = column.get();
    let was_find = current.mode == spike_library::column::Mode::Find;
    match mode {
        spike_library::column::Mode::Notes => {
            column.set(spike_library::column::enter_notes(&current))
        }
        spike_library::column::Mode::Marks => {
            let (next, ink) = spike_library::column::enter_marks(&current, &note.get());
            column.set(load_marks(&next));
            note.set(ink);
        }
        spike_library::column::Mode::Find => {
            let (next, ink) = spike_library::column::enter_find(&current, &note.get(), &live.get());
            column.set(next);
            note.set(ink);
        }
    }
    if was_find && mode != spike_library::column::Mode::Find {
        stage.command("find-stop", serde_json::json!(null));
    }
}

fn column_body(
    column: Signal<spike_library::column::Column>,
    live: Signal<spike_library::margin::LivePage>,
    stage: ElementRef,
    rect: Rect,
    window_h: f64,
) -> AnyElement {
    let mode = column.get().mode;
    let top = rect.y + 44.0;
    let height = (window_h - top - 12.0).max(1.0);
    let mut body = view()
        .absolute()
        .left(px(rect.x as f32))
        .top(px(top as f32))
        .w(px(rect.w as f32))
        .h(px(height as f32))
        .px(px(12.0))
        .overflow_hidden()
        .flex()
        .flex_col();
    if mode == spike_library::column::Mode::Marks {
        body = body.child(mark_list(column, live, stage));
    } else {
        body = body.child(find_list(column, live, stage));
    }
    body.into_element()
}

fn mark_list(
    column: Signal<spike_library::column::Column>,
    live: Signal<spike_library::margin::LivePage>,
    stage: ElementRef,
) -> AnyElement {
    let place = reading_place(&live.get());
    let current = column.get();
    let here = current
        .marks
        .iter()
        .any(|mark| spike_library::column::same_place(mark, place.page, &place.href, place.offset));
    let label = if here {
        "取消当前位置书签"
    } else {
        "添加当前位置书签"
    };
    let mut list = view().w_full().flex().flex_col().gap(px(8.0)).child(
        view()
            .on_press({
                let column = column.clone();
                let live = live.clone();
                move || {
                    let path = crate::settings::support_dir().join("annotations.sqlite");
                    let Ok(marks) = spike_library::column::Marks::open(&path) else {
                        return;
                    };
                    let place = reading_place(&live.get());
                    let label = spike_library::column::bookmark_label(
                        &place,
                        &crate::web::reading_excerpt(),
                    );
                    let Ok(next) = spike_library::column::toggle_mark(
                        &column.get(),
                        &marks,
                        &place,
                        &label,
                        now_ms(),
                    ) else {
                        return;
                    };
                    column.set(next);
                }
            })
            .child(
                line!(
                    label,
                    crate::tokens::BODY_SIZE,
                    crate::tokens::NAME_WEIGHT,
                    ink(),
                    0.0_f32
                )
                .line_height(px(18.0)),
            ),
    );
    if let Some(mark) = current.undo.clone() {
        list = list.child(
            view()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    line!(
                        "已删除书签",
                        crate::tokens::LABEL_SIZE,
                        crate::tokens::BODY_WEIGHT,
                        quiet(),
                        0.0_f32
                    )
                    .line_height(px(16.0)),
                )
                .child(
                    view()
                        .on_press({
                            let column = column.clone();
                            move || {
                                let path =
                                    crate::settings::support_dir().join("annotations.sqlite");
                                let Ok(marks) = spike_library::column::Marks::open(&path) else {
                                    return;
                                };
                                let Ok(next) =
                                    spike_library::column::undo_delete(&column.get(), &marks)
                                else {
                                    return;
                                };
                                column.set(next);
                            }
                        })
                        .child(
                            line!(
                                "撤销",
                                crate::tokens::LABEL_SIZE,
                                crate::tokens::NAME_WEIGHT,
                                ink(),
                                0.0_f32
                            )
                            .line_height(px(16.0)),
                        ),
                ),
        );
        let _ = mark;
    }
    for mark in current.marks {
        let at = spike_library::column::same_place(&mark, place.page, &place.href, place.offset);
        let id = mark.id;
        list = list.child(
            view()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .on_press({
                    let column = column.clone();
                    let stage = stage.clone();
                    let live = live.clone();
                    let mark = mark.clone();
                    move || {
                        let here = reading_place(&live.get());
                        let dest = spike_library::column::Place {
                            href: mark.href.clone(),
                            page: mark.page,
                            offset: mark.offset,
                        };
                        if dest != here {
                            let mut next = column.get();
                            next.find.navigated = true;
                            column.set(next);
                        }
                        stage.command(
                            "jump",
                            serde_json::json!({
                                "href": mark.href,
                                "page": mark.page,
                                "offset": mark.offset,
                                "end": mark.offset,
                            }),
                        );
                    }
                })
                .child(
                    line!(
                        mark.label.clone(),
                        crate::tokens::BODY_SIZE,
                        if at {
                            crate::tokens::NAME_WEIGHT
                        } else {
                            crate::tokens::BODY_WEIGHT
                        },
                        ink(),
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                )
                .child(
                    view()
                        .on_press({
                            let column = column.clone();
                            move || {
                                let path =
                                    crate::settings::support_dir().join("annotations.sqlite");
                                let Ok(marks) = spike_library::column::Marks::open(&path) else {
                                    return;
                                };
                                let Ok(next) = spike_library::column::delete_mark(
                                    &column.get(),
                                    &marks,
                                    id,
                                    now_ms(),
                                ) else {
                                    return;
                                };
                                column.set(next);
                            }
                        })
                        .child(
                            line!(
                                "×",
                                crate::tokens::BODY_SIZE,
                                crate::tokens::BODY_WEIGHT,
                                quiet(),
                                0.0_f32
                            )
                            .line_height(px(18.0)),
                        ),
                ),
        );
    }
    list.into_element()
}

fn find_list(
    column: Signal<spike_library::column::Column>,
    live: Signal<spike_library::margin::LivePage>,
    stage: ElementRef,
) -> AnyElement {
    let current = column.get();
    let line = spike_library::column::find_line(&current);
    let focus = current.find.focus;
    let mut list = view().w_full().flex().flex_col().gap(px(8.0)).child(
        nsh_core::element::input()
            .key(format!("book-find-{focus}"))
            .auto_focus(current.mode == spike_library::column::Mode::Find)
            .w_full()
            .h(px(28.0))
            .font_family(SharedString::from(crate::tokens::FONT_STACK))
            .text_size(px(crate::tokens::BODY_SIZE))
            .font_weight(crate::tokens::BODY_WEIGHT)
            .text_color(ink())
            .placeholder_color(quiet())
            .placeholder("查找正文")
            .auto_correct(false)
            .bg(transparent_black())
            .submit_behavior(SubmitBehavior::Submit)
            .value({
                let column = column.clone();
                move || column.get().find.draft.clone()
            })
            .on_change({
                let column = column.clone();
                let stage = stage.clone();
                move |text: &str| {
                    let before = column.get();
                    let next = spike_library::column::type_find(&before, text);
                    if next.find.generation != before.find.generation {
                        send_find(&stage, &next);
                    }
                    column.set(next);
                }
            })
            .on_composition_change({
                let column = column.clone();
                let stage = stage.clone();
                move |event: &Composition| {
                    let before = column.get();
                    let next = spike_library::column::compose_find(&before, event.composing);
                    if next.find.generation != before.find.generation {
                        send_find(&stage, &next);
                    }
                    column.set(next);
                }
            })
            .on_submit({
                let column = column.clone();
                let live = live.clone();
                let stage = stage.clone();
                move |text: &str| {
                    let mut current = column.get();
                    if current.find.draft != text && !current.find.composing {
                        current = spike_library::column::type_find(&current, text);
                    }
                    let place = reading_place(&live.get());
                    let generation = current.find.generation;
                    let pending = current.find.pending;
                    let (next, hit) = spike_library::column::find_return(&current, &place);
                    if next.find.generation != generation || next.find.pending && !pending {
                        send_find(&stage, &next);
                    }
                    if let Some(hit) = hit {
                        send_jump(&stage, &hit);
                    }
                    column.set(next);
                }
            }),
    );
    if !line.text.is_empty() {
        list = list.child(
            line!(
                line.text.clone(),
                crate::tokens::LABEL_SIZE,
                crate::tokens::BODY_WEIGHT,
                quiet(),
                0.0_f32
            )
            .line_height(px(16.0)),
        );
    }
    if line.retry {
        list = list.child(
            view()
                .on_press({
                    let column = column.clone();
                    let stage = stage.clone();
                    move || {
                        let next = spike_library::column::retry_find(&column.get());
                        send_find(&stage, &next);
                        column.set(next);
                    }
                })
                .child(
                    line!(
                        "重试",
                        crate::tokens::BODY_SIZE,
                        crate::tokens::NAME_WEIGHT,
                        ink(),
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                ),
        );
    }
    for (index, hit) in current.find.hits.iter().enumerate() {
        let hit = hit.clone();
        let chosen = current.find.index == Some(index);
        list = list.child(
            view()
                .on_press({
                    let column = column.clone();
                    let live = live.clone();
                    let stage = stage.clone();
                    move || {
                        let place = reading_place(&live.get());
                        let (next, hit) =
                            spike_library::column::choose(&column.get(), index, &place);
                        if let Some(hit) = hit {
                            send_jump(&stage, &hit);
                        }
                        column.set(next);
                    }
                })
                .child(
                    line!(
                        hit.snippet.clone(),
                        crate::tokens::BODY_SIZE,
                        if chosen {
                            crate::tokens::NAME_WEIGHT
                        } else {
                            crate::tokens::BODY_WEIGHT
                        },
                        ink(),
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                ),
        );
    }
    if current.find.composing {
        list.into_element()
    } else {
        FIND_STEP.with(|slot| {
            slot.replace(Some((column, live, stage)));
        });
        view()
            .w_full()
            .flex()
            .flex_col()
            .child(find_navigation_keys())
            .child(list)
            .into_element()
    }
}

pub(crate) struct MarginView {
    pub hold: spike_library::margin::Hold,
    pub fit: spike_library::margin::Fit,
    pub scene: spike_library::margin::Scene,
}

pub(crate) fn settle_margin(
    hold: &spike_library::margin::Hold,
    live: &spike_library::margin::LivePage,
    ease_moving: bool,
    page_box: &Rect,
    margin_x: f64,
    window_h: f64,
    notes: &[spike_library::margin::StoredNote],
    editor: Option<(i64, f64)>,
) -> MarginView {
    let mut live_now = live.clone();
    live_now.moving = live_now.moving || ease_moving;
    let mut next = spike_library::margin::reconcile(hold, &live_now);
    let fit = margin_fit(page_box, live_now.page_w, live_now.page_h);
    let mut cards = spike_library::margin::cards_from(notes, fit);
    if let Some((id, height)) = editor {
        if let Some(card) = cards.iter_mut().find(|card| card.id == id) {
            card.selected_h = height;
        }
    }
    let scene = spike_library::margin::place(
        &cards,
        spike_library::margin::Focus {
            hover: next.hover,
            selected: next.selected,
        },
        window_h,
        margin_x,
        next.scroll,
    );
    next.scroll = scene.scroll;
    next.scroll_min = scene.scroll_min;
    next.scroll_max = scene.scroll_max;
    MarginView {
        hold: next,
        fit,
        scene,
    }
}

fn margin_fit(page: &Rect, page_w: f64, page_h: f64) -> spike_library::margin::Fit {
    if page_w < 1.0 || page_h < 1.0 {
        return spike_library::margin::Fit {
            origin_x: 0.0,
            origin_y: 0.0,
            scale: 0.0,
        };
    }
    let (x, y, w, _) =
        spike_report::fitted_pdf_page(page.x, page.y, page.w, page.h, page_w, page_h);
    spike_library::margin::Fit {
        origin_x: x,
        origin_y: y,
        scale: w / page_w,
    }
}

pub(crate) fn commit_margin(
    field: &spike_library::margin::NoteField,
    submitted: &str,
    visible: &spike_library::margin::Visible,
) -> spike_library::margin::NoteField {
    let path = crate::settings::support_dir().join("annotations.sqlite");
    spike_library::margin::on_return(
        spike_library::margin::ReturnKind::Key,
        field,
        submitted,
        visible,
        &path,
    )
}

/// The annotation column is on screen, so its inputs are in the tree.
pub(crate) fn margin_column_open(form: Form) -> bool {
    matches!(
        form,
        Form::PageAndPanel {
            margin: Presence::Shown,
            ..
        }
    )
}

/// What the 批注 button does to the form and the drafts. Hiding the column
/// ends composition. The text stays.
pub(crate) fn press_margin_toggle(
    form: Form,
    ink: &spike_library::margin::MarginInk,
) -> (Form, spike_library::margin::MarginInk) {
    let next = step(form, Command::TogglePanel);
    let ink = if margin_column_open(form) && !margin_column_open(next) {
        spike_library::margin::release_composition(ink)
    } else {
        ink.clone()
    };
    (next, ink)
}

pub(crate) fn commit_ink(
    ink: &spike_library::margin::MarginInk,
    submitted: &str,
    visible: &spike_library::margin::Visible,
) -> spike_library::margin::MarginInk {
    let path = crate::settings::support_dir().join("annotations.sqlite");
    spike_library::margin::submit_new(ink, submitted, visible, &path)
}

pub(crate) fn commit_card(
    ink: &spike_library::margin::MarginInk,
    key: &spike_library::margin::CardKey,
    submitted: &str,
) -> spike_library::margin::MarginInk {
    let path = crate::settings::support_dir().join("annotations.sqlite");
    spike_library::margin::submit_card(ink, key, submitted, &path).0
}

pub(crate) fn absorb_margin(
    ink: &spike_library::margin::MarginInk,
    value: &serde_json::Value,
    live: &spike_library::margin::LivePage,
) -> spike_library::margin::MarginInk {
    let moved = spike_library::margin::retarget(ink, &live.book, &live.spine, live.page);
    let field = note_from_status(&moved.new_note, value, &live.book);
    spike_library::margin::MarginInk {
        new_note: field,
        ..moved
    }
}

pub(crate) fn note_from_status(
    field: &spike_library::margin::NoteField,
    value: &serde_json::Value,
    book: &str,
) -> spike_library::margin::NoteField {
    let gen = value
        .get("book_gen")
        .and_then(|gen| gen.as_u64())
        .unwrap_or(0);
    let published = value
        .get("passage")
        .map(crate::visit::passage_from_value)
        .unwrap_or(None);
    let taken = spike_library::margin::take_passage(
        published,
        &spike_library::margin::LiveBook {
            book: book.to_string(),
            gen,
        },
    );
    if field.passage == taken {
        return field.clone();
    }
    spike_library::margin::NoteField {
        passage: taken,
        fingerprint: None,
        ..field.clone()
    }
}

fn card_editor_height(
    ink: &spike_library::margin::MarginInk,
    notes: &[spike_library::margin::StoredNote],
    book: &str,
) -> Option<(i64, f64)> {
    let spike_library::margin::Face::Card(key) = &ink.face else {
        return None;
    };
    if key.book != book {
        return None;
    }
    let note = notes.iter().find(|note| note.id == key.note_id)?;
    let draft = ink
        .draft(key)
        .map(|draft| draft.body.as_str())
        .unwrap_or("");
    let quote = if note.passage.trim().is_empty() {
        spike_library::margin::shown_text(note)
    } else {
        note.passage.as_str()
    };
    let height = match key.kind {
        spike_library::margin::CardKind::Edit => spike_library::margin::edit_card_h(quote, draft),
        spike_library::margin::CardKind::Reply => {
            spike_library::margin::reply_card_h(quote, &note.replies, draft)
        }
    };
    Some((note.id, height))
}

fn margin_composer(
    note: Signal<spike_library::margin::MarginInk>,
    live: Signal<spike_library::margin::LivePage>,
    book_gen: Signal<u64>,
    rect: Rect,
    composer_top: f64,
) -> AnyElement {
    let held = note.get();
    let mut row = view()
        .absolute()
        .left(px(
            (rect.x + f64::from(crate::tokens::COMPOSER_INSET)) as f32
        ))
        .top(px(composer_top as f32))
        .w(px(
            (rect.w - f64::from(crate::tokens::COMPOSER_INSET) * 2.0).max(1.0) as f32,
        ))
        .h(px(crate::tokens::MARGIN_COMPOSER_H))
        .rounded(px(crate::tokens::COMPOSER_RADIUS))
        .bg(rgb(crate::tokens::CARD).to_hsla())
        .shadow(crate::tokens::margin_ring())
        .px(px(crate::tokens::COMPOSER_PAD))
        .flex()
        .flex_col()
        .justify_center();
    if matches!(held.face, spike_library::margin::Face::Card(_)) {
        return row
            .pointer_events(PointerEvents::BoxNone)
            .child(
                line!(
                    "新建已收起",
                    crate::tokens::BODY_SIZE,
                    crate::tokens::BODY_WEIGHT,
                    quiet(),
                    0.0_f32
                )
                .line_height(px(18.0)),
            )
            .into_element();
    }
    if held
        .parked
        .as_ref()
        .is_some_and(|parked| !parked.body.trim().is_empty())
    {
        let restore = note.clone();
        row = row.child(
            view()
                .on_press(move || {
                    let next = spike_library::margin::restore_new(&restore.get());
                    if next != restore.get() {
                        restore.set(next);
                    }
                })
                .child(
                    line!(
                        "恢复新建草稿",
                        crate::tokens::BODY_SIZE,
                        crate::tokens::NAME_WEIGHT,
                        ink(),
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                ),
        );
    }
    row.pointer_events(PointerEvents::Auto)
        .child(margin_note_input(note, live, book_gen))
        .into_element()
}

fn margin_note_input(
    note: Signal<spike_library::margin::MarginInk>,
    live: Signal<spike_library::margin::LivePage>,
    book_gen: Signal<u64>,
) -> AnyElement {
    let write = note.clone();
    let compose = note.clone();
    let submit = note.clone();
    let submit_live = live.clone();
    let submit_gen = book_gen.clone();
    let focus = note.get().focus_new;
    nsh_core::element::input()
        .key(format!("margin-note-{focus}"))
        .auto_focus(focus > 0)
        .flex_1()
        .min_w(px(0.0))
        .font_family(SharedString::from(crate::tokens::FONT_STACK))
        .text_size(px(crate::tokens::BODY_SIZE))
        .font_weight(crate::tokens::BODY_WEIGHT)
        .text_color(ink())
        .placeholder_color(quiet())
        .placeholder("笔记")
        .auto_correct(false)
        .bg(transparent_black())
        .submit_behavior(SubmitBehavior::Submit)
        // NSH `return_key` returns before submit while `hasMarkedText` is set.
        // This field's composition flag is the second guard. AppKit was not run.
        .value(move || note.get().new_note.body.clone())
        .on_change(move |text: &str| {
            let next = spike_library::margin::type_new(&write.get(), text);
            if next != write.get() {
                write.set(next);
            }
        })
        .on_composition_change(move |event: &Composition| {
            let next = spike_library::margin::compose_new(&compose.get(), event.composing);
            if next != compose.get() {
                compose.set(next);
            }
        })
        .on_submit(move |text: &str| {
            let current = submit_live.get();
            let next = commit_ink(
                &submit.get(),
                text,
                &spike_library::margin::Visible {
                    book: spike_library::margin::LiveBook {
                        book: current.book,
                        gen: submit_gen.get(),
                    },
                    page: current.page,
                    spine: current.spine,
                    moving: current.moving,
                },
            );
            submit.set(next);
        })
        .into_element()
}

pub(crate) fn load_margin(
    book: &str,
    page: u32,
    spine: &str,
) -> Result<Vec<spike_library::margin::StoredNote>, String> {
    if book.is_empty() || page == 0 {
        return Ok(Vec::new());
    }
    let path = crate::settings::support_dir().join("annotations.sqlite");
    let notes = spike_library::margin::Notes::open(&path)?;
    notes.list(book, page, spine)
}

pub(crate) fn load_spine(
    book: &str,
    spine: &str,
) -> Result<Vec<spike_library::margin::StoredNote>, String> {
    if book.is_empty() || spine.is_empty() {
        return Ok(Vec::new());
    }
    let path = crate::settings::support_dir().join("annotations.sqlite");
    let notes = spike_library::margin::Notes::open(&path)?;
    notes.list_spine(book, spine)
}

fn margin_action(label: &'static str, press: impl Fn() + 'static) -> AnyElement {
    view()
        .mr(px(12.0))
        .on_press(move || press())
        .child(
            line!(
                label,
                crate::tokens::BODY_SIZE,
                crate::tokens::NAME_WEIGHT,
                ink(),
                0.0_f32
            )
            .line_height(px(18.0)),
        )
        .into_element()
}

fn margin_card_field(
    margin: Signal<spike_library::margin::MarginInk>,
    key: spike_library::margin::CardKey,
    placeholder: &'static str,
) -> AnyElement {
    let write = margin.clone();
    let compose = margin.clone();
    let submit = margin.clone();
    let cancel = margin.clone();
    let save = margin.clone();
    let key_change = key.clone();
    let key_compose = key.clone();
    let key_submit = key.clone();
    let key_cancel = key.clone();
    let key_save = key.clone();
    let body = margin
        .get()
        .draft(&key)
        .map(|draft| draft.body.clone())
        .unwrap_or_default();
    let lines = spike_library::margin::editor_field_h(&body);
    let mut column = view().w_full().flex().flex_col().child(
        nsh_core::element::input()
            .key(format!(
                "card-{}-{}",
                key.note_id,
                matches!(key.kind, spike_library::margin::CardKind::Reply)
            ))
            .auto_focus(true)
            .w_full()
            .h(px(lines as f32))
            .multiline(true)
            .font_family(SharedString::from(crate::tokens::FONT_STACK))
            .text_size(px(crate::tokens::BODY_SIZE))
            .font_weight(crate::tokens::BODY_WEIGHT)
            .text_color(ink())
            .placeholder_color(quiet())
            .placeholder(placeholder)
            .auto_correct(false)
            .bg(transparent_black())
            .submit_behavior(SubmitBehavior::Submit)
            .value({
                let write = write.clone();
                let key_change = key_change.clone();
                move || {
                    write
                        .get()
                        .draft(&key_change)
                        .map(|draft| draft.body.clone())
                        .unwrap_or_default()
                }
            })
            .on_change({
                let write = write.clone();
                let key_change = key_change.clone();
                move |text: &str| {
                    let next = spike_library::margin::type_card(&write.get(), &key_change, text);
                    if next != write.get() {
                        write.set(next);
                    }
                }
            })
            .on_composition_change(move |event: &Composition| {
                let next = spike_library::margin::compose_card(
                    &compose.get(),
                    &key_compose,
                    event.composing,
                );
                if next != compose.get() {
                    compose.set(next);
                }
            })
            .on_key_press({
                let slot = cancel.clone();
                let key = key_cancel.clone();
                move |pressed: &str| {
                    if pressed == "Escape" || pressed == "escape" {
                        let next = spike_library::margin::cancel_card(&slot.get(), &key);
                        if next != slot.get() {
                            slot.set(next);
                        }
                    }
                }
            })
            .on_submit(move |text: &str| {
                let next = commit_card(&submit.get(), &key_submit, text);
                submit.set(next);
            }),
    );
    if let Some(notice) = margin.get().draft(&key).map(|draft| draft.notice.clone()) {
        if !notice.is_empty() {
            column = column.child(
                line!(
                    notice,
                    crate::tokens::BODY_SIZE,
                    crate::tokens::BODY_WEIGHT,
                    quiet(),
                    0.0_f32
                )
                .line_height(px(18.0)),
            );
        }
    }
    column
        .child(
            view()
                .flex()
                .flex_row()
                .mt(px(8.0))
                .child(margin_action("取消", move || {
                    let next = spike_library::margin::cancel_card(&cancel.get(), &key_cancel);
                    if next != cancel.get() {
                        cancel.set(next);
                    }
                }))
                .child(margin_action("保存", move || {
                    let current = save.get();
                    let text = current
                        .draft(&key_save)
                        .map(|draft| draft.body.clone())
                        .unwrap_or_default();
                    save.set(commit_card(&current, &key_save, &text));
                })),
        )
        .into_element()
}

fn margin_card(
    note: &spike_library::margin::StoredNote,
    placed: &spike_library::margin::Placed,
    hold: Signal<spike_library::margin::Hold>,
    margin: Signal<spike_library::margin::MarginInk>,
    book: &str,
) -> AnyElement {
    let id = note.id;
    let source = spike_library::margin::shown_text(note).to_string();
    let sentence = source.lines().next().unwrap_or("").trim().to_string();
    let quote = if note.passage.trim().is_empty() {
        source.clone()
    } else {
        note.passage.clone()
    };
    let open_key = match margin.get().face.clone() {
        spike_library::margin::Face::Card(key) if key.note_id == id && key.book == book => {
            Some(key)
        }
        _ => None,
    };
    let mut column = view().w_full().flex().flex_col().child(
        line!(
            note.author.clone(),
            crate::tokens::BODY_SIZE,
            crate::tokens::NAME_WEIGHT,
            ink(),
            0.0_f32
        )
        .line_height(px(18.0))
        .truncate(),
    );
    if let Some(key) = open_key.clone() {
        match key.kind {
            spike_library::margin::CardKind::Edit => {
                column = column.child(
                    line!(
                        quote.clone(),
                        crate::tokens::BODY_SIZE,
                        crate::tokens::BODY_WEIGHT,
                        quiet(),
                        0.0_f32
                    )
                    .line_height(px(18.0)),
                );
            }
            spike_library::margin::CardKind::Reply => {
                column = column
                    .child(
                        line!(
                            "回复这条批注",
                            crate::tokens::BODY_SIZE,
                            crate::tokens::NAME_WEIGHT,
                            ink(),
                            0.0_f32
                        )
                        .line_height(px(18.0)),
                    )
                    .child(
                        line!(
                            quote.clone(),
                            crate::tokens::BODY_SIZE,
                            crate::tokens::BODY_WEIGHT,
                            quiet(),
                            0.0_f32
                        )
                        .line_height(px(18.0)),
                    );
                for reply in &note.replies {
                    column = column.child(
                        line!(
                            reply.clone(),
                            crate::tokens::BODY_SIZE,
                            crate::tokens::BODY_WEIGHT,
                            ink(),
                            0.0_f32
                        )
                        .line_height(px(18.0))
                        .mt(px(crate::tokens::MARGIN_CARD_GAP + 8.0)),
                    );
                }
            }
        }
        let placeholder = match key.kind {
            spike_library::margin::CardKind::Edit => "笔记",
            spike_library::margin::CardKind::Reply => "回复",
        };
        column = column.child(margin_card_field(margin.clone(), key, placeholder));
    } else if placed.open {
        let seed = source.clone();
        column = column.child(
            line!(
                source,
                crate::tokens::BODY_SIZE,
                crate::tokens::BODY_WEIGHT,
                ink(),
                0.0_f32
            )
            .line_height(px(18.0)),
        );
        if hold.get().selected == Some(id) {
            for reply in &note.replies {
                column = column.child(
                    line!(
                        reply.clone(),
                        crate::tokens::BODY_SIZE,
                        crate::tokens::BODY_WEIGHT,
                        ink(),
                        0.0_f32
                    )
                    .line_height(px(18.0))
                    .mt(px(crate::tokens::MARGIN_CARD_GAP + 8.0)),
                );
            }
            let edit_hold = hold.clone();
            let edit_ink = margin.clone();
            let reply_hold = hold.clone();
            let reply_ink = margin.clone();
            let book_edit = book.to_string();
            let book_reply = book.to_string();
            column = column.child(
                view()
                    .flex()
                    .flex_row()
                    .mt(px(8.0))
                    .child(margin_action("编辑", move || {
                        let mut current = edit_hold.get();
                        current.selected = Some(id);
                        edit_hold.set(current);
                        let key = spike_library::margin::CardKey {
                            book: book_edit.clone(),
                            note_id: id,
                            kind: spike_library::margin::CardKind::Edit,
                        };
                        edit_ink.set(spike_library::margin::begin_card(
                            &edit_ink.get(),
                            key,
                            &seed,
                        ));
                    }))
                    .child(margin_action("回复", move || {
                        let mut current = reply_hold.get();
                        current.selected = Some(id);
                        reply_hold.set(current);
                        let key = spike_library::margin::CardKey {
                            book: book_reply.clone(),
                            note_id: id,
                            kind: spike_library::margin::CardKind::Reply,
                        };
                        reply_ink.set(spike_library::margin::begin_card(&reply_ink.get(), key, ""));
                    })),
            );
        }
    } else {
        column = column.child(
            line!(
                sentence,
                crate::tokens::BODY_SIZE,
                crate::tokens::BODY_WEIGHT,
                ink(),
                0.0_f32
            )
            .line_height(px(18.0))
            .truncate(),
        );
    }
    let top = (placed.y - spike_library::margin::BAND_TOP) as f32;
    let editing = open_key.is_some();
    view()
        .absolute()
        .left(px(crate::tokens::MARGIN_CARD_X))
        .top(px(top))
        .w(px(crate::tokens::MARGIN_CARD_W))
        .h(px(placed.h as f32))
        .rounded(px(crate::tokens::CARD_RADIUS as f32))
        .bg(rgb(crate::tokens::CARD).to_hsla())
        .shadow(crate::tokens::margin_ring())
        .overflow_hidden()
        .px(px(12.0))
        .pt(px(8.0))
        .pb(px(8.0))
        .pointer_events(PointerEvents::Auto)
        .on_mouse_enter({
            let hold = hold.clone();
            move |_: &MouseEvent| {
                let mut current = hold.get();
                if current.hover != Some(id) {
                    current.hover = Some(id);
                    hold.set(current);
                }
            }
        })
        .on_mouse_leave({
            let hold = hold.clone();
            move |_: &MouseEvent| {
                let mut current = hold.get();
                if current.hover == Some(id) {
                    current.hover = None;
                    hold.set(current);
                }
            }
        })
        .on_press({
            let hold = hold.clone();
            let margin = margin.clone();
            move || {
                if editing {
                    return;
                }
                if let spike_library::margin::Face::Card(key) = margin.get().face.clone() {
                    if key.note_id != id {
                        margin.set(spike_library::margin::park_card(&margin.get()));
                    }
                }
                let leaving = hold.get().selected == Some(id);
                if leaving {
                    margin.set(spike_library::margin::park_card(&margin.get()));
                }
                let mut current = hold.get();
                current.selected = if leaving { None } else { Some(id) };
                current.scroll = 0.0;
                hold.set(current);
            }
        })
        .child(column)
        .into_element()
}

fn immersive_button(form: Signal<Form>, seat: Rect, plate: bool, stage: ElementRef) -> AnyElement {
    let mut button = view()
        .absolute()
        .left(px(seat.x as f32))
        .top(px(seat.y as f32))
        .w(px(seat.w as f32))
        .h(px(seat.h as f32))
        .rounded(px(crate::tokens::TOGGLE_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("Immersive Reading");
    if plate {
        button = button.bg(wash(crate::tokens::ROW_ON_ALPHA));
    }
    button = button.on_press(move || {
        apply_immersive(&form, true, false, &stage);
    });
    if let Some(path) = crate::icons::glyph_ink("maximize", "6E6E73") {
        button = button.child(image(path).size(px(crate::tokens::IMMERSIVE_ICON)));
    }
    button.into_element()
}

fn margin_toggle(
    form: Signal<Form>,
    chrome: crate::tokens::ChromeIn,
    stage: ElementRef,
    note: Signal<spike_library::margin::MarginInk>,
) -> AnyElement {
    let top = chrome.top.max(0.0) + crate::tokens::TOGGLE_TOP;
    let mut button = view()
        .absolute()
        .right(px(crate::tokens::CHAT_PAD_X))
        .top(px(top))
        .w(px(crate::tokens::TOGGLE_SIZE))
        .h(px(crate::tokens::TOGGLE_SIZE))
        .rounded(px(crate::tokens::TOGGLE_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("批注")
        .on_press(move || {
            let (next, released) = press_margin_toggle(form.get(), &note.get());
            if released != note.get() {
                note.set(released);
            }
            let shown = margin_column_open(next);
            form.set(next);
            publish_margin(shown);
            stage.command("margin", serde_json::json!(shown));
        });
    if let Some(path) = crate::icons::glyph_ink("sidebar-right", "6E6E73") {
        button = button.child(image(path).size(px(crate::tokens::PANEL_ICON)));
    }
    button.into_element()
}

fn panel_toggle_button(
    form: Signal<Form>,
    floating: bool,
    row: crate::tokens::ChromeRow,
) -> AnyElement {
    let mut button = view()
        .w(px(crate::tokens::TOGGLE_SIZE))
        .h(px(crate::tokens::TOGGLE_SIZE))
        .rounded(px(crate::tokens::TOGGLE_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("对话")
        .on_press(move || {
            form.set(panel_command(form.get(), floating));
        });
    if floating {
        button = button
            .absolute()
            .right(px(crate::tokens::CHAT_PAD_X))
            .top(px(row.toggle_y));
    }
    if let Some(path) = crate::icons::glyph_ink("sidebar-right", "6E6E73") {
        button = button.child(image(path).size(px(crate::tokens::PANEL_ICON)));
    }
    button.into_element()
}

fn search_field(query: Signal<String>) -> AnyElement {
    let write = query.clone();
    view()
        .w_full()
        .min_w(px(0.0))
        .mb(px(crate::tokens::SEARCH_MB))
        .h(px(crate::tokens::SEARCH_H))
        .rounded(px(crate::tokens::SEARCH_RADIUS))
        .bg(wash(crate::tokens::SEARCH_ALPHA))
        .px(px(crate::tokens::SEARCH_PAD_X))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(crate::tokens::SEARCH_GAP))
        .child(glyph_box("search", false, crate::tokens::ROW_ICON, None))
        .child(
            nsh_core::element::input()
                .key("shelf-search")
                .flex_1()
                .min_w(px(0.0))
                .font_family(SharedString::from(crate::tokens::FONT_STACK))
                .text_size(px(crate::tokens::SEARCH_SIZE))
                .font_weight(crate::tokens::BODY_WEIGHT)
                .text_color(quiet())
                .placeholder_color(quiet())
                .placeholder("搜索书名、作者")
                .auto_correct(false)
                .bg(transparent_black())
                .value(move || query.get())
                .on_change(move |text: &str| write.set(text.to_string())),
        )
        .into_element()
}

fn group_label(label: &str) -> AnyElement {
    view()
        .w_full()
        .min_w(px(0.0))
        .pt(px(crate::tokens::LABEL_MT))
        .pb(px(crate::tokens::LABEL_MB))
        .child(fit_text(
            label.to_string(),
            crate::tokens::LABEL_SIZE,
            crate::tokens::LABEL_WEIGHT,
            quiet(),
            crate::tokens::LABEL_TRACK_EM,
        ))
        .into_element()
}

fn side_row(row: SideRow, selected: Signal<SelectId>, menu: Signal<bool>) -> AnyElement {
    let id = row.id.clone();
    let dotted = row.dot.is_some();
    let add = matches!(row.id, SelectId::AddSource);
    let weight = if row.selected {
        crate::tokens::ROW_WEIGHT_ON
    } else {
        crate::tokens::BODY_WEIGHT
    };
    let color = if add { quiet() } else { ink() };
    let mut line = view()
        .h(px(crate::tokens::ROW_H))
        .rounded(px(crate::tokens::ROW_RADIUS))
        .pl(px(crate::tokens::ROW_PAD_L))
        .pr(px(crate::tokens::ROW_PAD_R))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(crate::tokens::ROW_GAP))
        .accessible(true)
        .label(row.label.clone())
        .on_press(move || {
            if add {
                menu.set(!menu.get());
                return;
            }
            if dotted {
                return;
            }
            menu.set(false);
            selected.set(id.clone());
        });
    if row.selected {
        line = line.bg(wash(crate::tokens::ROW_ON_ALPHA));
    }
    line = line.child(row_mark(&row));
    line = line.child(view().flex_1().min_w(px(0.0)).child(fit_text(
        row.label.clone(),
        crate::tokens::BODY_SIZE,
        weight,
        color,
        crate::tokens::BODY_TRACK_EM,
    )));
    if let Some(count) = row.count {
        line = line.child(line!(
            count.to_string(),
            crate::tokens::COUNT_ROW_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM
        ));
    }
    line.into_element()
}

fn row_mark(row: &SideRow) -> AnyElement {
    if let Some(color) = row.dot {
        return view()
            .w(px(crate::tokens::ROW_ICON_SLOT))
            .flex()
            .items_center()
            .justify_center()
            .child(
                view()
                    .size(px(crate::tokens::DOT))
                    .rounded(px(crate::tokens::DOT / 2.0))
                    .bg(rgb(color).to_hsla()),
            )
            .into_element();
    }
    if let Some((from, to)) = row.badge {
        return view()
            .w(px(crate::tokens::ROW_ICON_SLOT))
            .flex()
            .items_center()
            .justify_center()
            .child(
                view()
                    .size(px(crate::tokens::SOURCE_BADGE))
                    .rounded(px(crate::tokens::SOURCE_BADGE_RADIUS))
                    .bg_linear_to_b()
                    .gradient_from(rgb(from).to_hsla())
                    .gradient_to(rgb(to).to_hsla())
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(glyph_box(
                        row.icon,
                        false,
                        crate::tokens::SOURCE_GLYPH,
                        Some("FFFFFF"),
                    )),
            )
            .into_element();
    }
    let size = if row.icon == "plus" {
        crate::tokens::ADD_ICON
    } else if row.id == SelectId::Settings {
        crate::tokens::SETTINGS_ICON
    } else {
        crate::tokens::ROW_ICON
    };
    let hex = if row.selected {
        "1D1D1F"
    } else if matches!(row.id, SelectId::AddSource) {
        "8E8E93"
    } else {
        "6E6E73"
    };
    view()
        .w(px(crate::tokens::ROW_ICON_SLOT))
        .flex()
        .items_center()
        .justify_center()
        .child(glyph_box(row.icon, row.selected, size, Some(hex)))
        .into_element()
}

fn glyph_box(name: &str, solid: bool, size: f32, hex: Option<&str>) -> AnyElement {
    if name.is_empty() {
        return view().size(px(size)).into_element();
    }
    let key = if solid {
        format!("{name}-solid")
    } else {
        name.to_string()
    };
    let ink = hex.unwrap_or("1D1D1F");
    let path = crate::icons::glyph_ink(&key, ink).or_else(|| crate::icons::glyph_ink(name, ink));
    match path {
        Some(path) => image(path).size(px(size)).into_element(),
        None => view().size(px(size)).into_element(),
    }
}

fn chat_pane(rect: Option<Rect>, form: Signal<Form>, library: &LibraryIn) -> AnyElement {
    let chip = library
        .continue_reading
        .first()
        .map(|card| card.title.clone())
        .unwrap_or_default();
    let mut pane = view().w(px(CHAT_W as f32)).h_full().flex().flex_col();
    if let Some(rect) = rect {
        pane = pane
            .absolute()
            .left(px(rect.x as f32))
            .top(px(rect.y as f32))
            .w(px(rect.w as f32))
            .h(px(rect.h as f32));
    }
    pane.child(chat_head(form))
        .child(chat_thread(library))
        .child(composer(&chip))
        .into_element()
}

fn chat_head(form: Signal<Form>) -> AnyElement {
    view()
        .h(px(crate::tokens::CHAT_HEAD_H))
        .px(px(crate::tokens::CHAT_PAD_X))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(crate::tokens::CHAT_HEAD_GAP))
        .child(avatar())
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_row()
                .items_baseline()
                .gap(px(crate::tokens::NAME_GAP))
                .child(view().flex_shrink(1.0).min_w(px(0.0)).child(fit_text(
                    "Slowpage".to_string(),
                    crate::tokens::NAME_SIZE,
                    crate::tokens::NAME_WEIGHT,
                    ink(),
                    crate::tokens::BODY_TRACK_EM,
                )))
                .child(view().flex_1().min_w(px(0.0)).child(fit_text(
                    "你的阅读助手".to_string(),
                    crate::tokens::SUB_SIZE,
                    crate::tokens::BODY_WEIGHT,
                    quiet(),
                    crate::tokens::BODY_TRACK_EM,
                ))),
        )
        .child(
            view()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(crate::tokens::TOOL_CLUSTER_GAP))
                .child(icon_button("edit", "新对话", crate::tokens::PANEL_ICON))
                .child(panel_toggle_button(
                    form,
                    false,
                    crate::tokens::chrome_row(chrome_off()),
                )),
        )
        .into_element()
}

fn icon_button(name: &str, label: &str, size: f32) -> AnyElement {
    let mut button = view()
        .w(px(crate::tokens::TOGGLE_SIZE))
        .h(px(crate::tokens::TOGGLE_SIZE))
        .rounded(px(crate::tokens::TOGGLE_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label(label.to_string());
    if let Some(path) = crate::icons::glyph_ink(name, "6E6E73") {
        button = button.child(image(path).size(px(size)));
    }
    button.into_element()
}

fn avatar() -> AnyElement {
    view()
        .size(px(crate::tokens::AVATAR))
        .rounded(px(crate::tokens::AVATAR / 2.0))
        .bg_linear_to_br()
        .gradient_from(rgb(crate::tokens::AVATAR_FROM).to_hsla())
        .gradient_to(rgb(crate::tokens::AVATAR_TO).to_hsla())
        .flex()
        .items_center()
        .justify_center()
        .child(line!(
            "慢",
            crate::tokens::AVATAR_MARK_SIZE,
            crate::tokens::AVATAR_MARK_WEIGHT,
            rgb(crate::tokens::CARD).to_hsla(),
            0.0
        ))
        .into_element()
}

fn chat_thread(library: &LibraryIn) -> AnyElement {
    let mut column = view()
        .flex_1()
        .flex()
        .flex_col()
        .px(px(crate::tokens::CHAT_PAD_X))
        .pt(px(crate::tokens::CONV_PT))
        .pb(px(crate::tokens::COMPOSER_CLEAR))
        .gap(px(crate::tokens::CONV_GAP));
    for line in &library.chat {
        column = column.child(chat_line(line));
    }
    column.into_element()
}

fn chat_line(line: &ChatLine) -> AnyElement {
    match line.kind.as_str() {
        "time" => view()
            .flex()
            .justify_center()
            .child(line!(
                line.text.clone(),
                crate::tokens::TIME_RULE_SIZE,
                crate::tokens::BODY_WEIGHT,
                faint(),
                crate::tokens::BODY_TRACK_EM
            ))
            .into_element(),
        "user" => view()
            .flex()
            .flex_row()
            .justify_end()
            .child(
                view()
                    .max_w(px(crate::tokens::BUBBLE_MAX))
                    .rounded(px(crate::tokens::BUBBLE_RADIUS))
                    .px(px(crate::tokens::BUBBLE_PX))
                    .py(px(crate::tokens::BUBBLE_PY))
                    .bg(rgb(crate::tokens::CARD).to_hsla())
                    .shadow(crate::tokens::bubble_shadows())
                    .child(line!(
                        line.text.clone(),
                        crate::tokens::MSG_SIZE,
                        crate::tokens::BODY_WEIGHT,
                        ink(),
                        crate::tokens::BODY_TRACK_EM
                    )),
            )
            .into_element(),
        "chips" => {
            let mut row = view()
                .w_full()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(crate::tokens::CHIP_GAP));
            for chip in line.text.split('\n').filter(|chip| !chip.is_empty()) {
                row = row.child(
                    view()
                        .rounded(px(crate::tokens::CHIP_RADIUS))
                        .px(px(crate::tokens::CHIP_PX))
                        .py(px(crate::tokens::CHIP_PY))
                        .bg(hsla(0.0, 0.0, 1.0, 0.55))
                        .shadow(crate::tokens::chip_shadows())
                        .child(line!(
                            chip.to_string(),
                            crate::tokens::CHIP_SIZE,
                            crate::tokens::BODY_WEIGHT,
                            rgb(crate::tokens::CHIP_INK).to_hsla(),
                            crate::tokens::BODY_TRACK_EM
                        )),
                );
            }
            row.into_element()
        }
        _ => line!(
            line.text.clone(),
            crate::tokens::MSG_SIZE,
            crate::tokens::BODY_WEIGHT,
            ink(),
            crate::tokens::BODY_TRACK_EM
        )
        .into_element(),
    }
}

fn composer(book: &str) -> AnyElement {
    let mut tools = view()
        .mt(px(crate::tokens::COMPOSER_ROW_MT))
        .h(px(crate::tokens::COMPOSER_CONTROL))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(crate::tokens::COMPOSER_ROW_GAP))
        .child(attach_button());
    if !book.is_empty() {
        tools = tools.child(scope_chip(book));
    }
    tools = tools.child(view().flex_1().min_w(px(0.0)));
    tools = tools.child(send_button());
    view()
        .absolute()
        .left(px(crate::tokens::COMPOSER_INSET))
        .right(px(crate::tokens::COMPOSER_INSET))
        .bottom(px(crate::tokens::COMPOSER_INSET))
        .rounded(px(crate::tokens::COMPOSER_RADIUS))
        .p(px(crate::tokens::COMPOSER_PAD))
        .bg(rgb(crate::tokens::CARD).to_hsla())
        .shadow(crate::tokens::composer_shadows())
        .accessible(true)
        .label("问问 Slowpage…")
        .child({
            remember_font(
                "问问 Slowpage…",
                crate::tokens::BODY_SIZE,
                crate::tokens::BODY_WEIGHT,
            );
            view().min_w(px(0.0)).child(fit_text(
                "问问 Slowpage…".to_string(),
                crate::tokens::BODY_SIZE,
                crate::tokens::BODY_WEIGHT,
                faint(),
                crate::tokens::BODY_TRACK_EM,
            ))
        })
        .child(tools)
        .into_element()
}

fn attach_button() -> AnyElement {
    let mut button = view()
        .w(px(crate::tokens::ATTACH_ICON))
        .h(px(crate::tokens::COMPOSER_CONTROL))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("附件");
    if let Some(path) = crate::icons::glyph_ink("paperclip", "6E6E73") {
        button = button.child(image(path).size(px(crate::tokens::ATTACH_ICON)));
    }
    button.into_element()
}

fn scope_chip(book: &str) -> AnyElement {
    let natural = scope_chip_width(book);
    let width = natural.min(chip_limit(CHAT_W as f32));
    view()
        .w(px(width))
        .h(px(crate::tokens::COMPOSER_CONTROL))
        .flex_shrink(0.0)
        .rounded(px(crate::tokens::SCOPE_RADIUS))
        .px(px(crate::tokens::SCOPE_PX))
        .gap(px(crate::tokens::SCOPE_GAP))
        .flex()
        .flex_row()
        .items_center()
        .bg(rgb(crate::tokens::SCOPE_BG).to_hsla())
        .child(glyph_box(
            "book-open",
            false,
            crate::tokens::SOURCE_GLYPH,
            Some("6E6E73"),
        ))
        .child(view().flex_1().min_w(px(0.0)).child(fit_text(
            book.to_string(),
            crate::tokens::SUB_SIZE,
            crate::tokens::BODY_WEIGHT,
            icon_ink(),
            crate::tokens::BODY_TRACK_EM,
        )))
        .into_element()
}

fn send_button() -> AnyElement {
    let mut button = view()
        .size(px(crate::tokens::SEND))
        .rounded(px(crate::tokens::SEND / 2.0))
        .bg(rgb(crate::tokens::SEND_BG).to_hsla())
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label("发送");
    if let Some(path) = crate::icons::glyph_ink("arrow-up", "FFFFFF") {
        button = button.child(image(path).size(px(crate::tokens::SEND_ICON)));
    }
    button.into_element()
}

fn card_face(
    rect: Option<Rect>,
    radius: f64,
    face: CardFace,
    library: &LibraryIn,
    on_peer: impl Fn(Peer) + Clone + 'static,
    on_open: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    let title = selected_title(&library.selected);
    let content_w = rect
        .map(|item| (item.w as f32 - crate::tokens::CARD_PAD_X * 2.0).max(0.0))
        .unwrap_or(0.0);
    let mut body = view()
        .flex_1()
        .rounded(px(radius as f32))
        .bg(rgb(crate::tokens::CARD).to_hsla())
        .shadow(crate::tokens::card_shadows())
        .overflow_hidden()
        .flex()
        .flex_col();
    if let Some(rect) = rect {
        body = body
            .absolute()
            .left(px(rect.x as f32))
            .top(px(rect.y as f32))
            .w(px(rect.w as f32))
            .h(px(rect.h as f32));
    }
    if !matches!(face, CardFace::Page) {
        body = body.child(card_toolbar());
    }
    let mut column = view()
        .flex_1()
        .flex()
        .flex_col()
        .px(px(crate::tokens::CARD_PAD_X))
        .pt(px(crate::tokens::CARD_PAD_TOP));
    if matches!(face, CardFace::Grid) {
        column = column.pb(px(crate::tokens::CARD_RADIUS as f32));
    }
    if !matches!(face, CardFace::Page) {
        column = column.child(card_header(&title, library.book_count));
    }
    if let Some(notice) = &library.notice {
        column = column.child(
            line!(
                notice.clone(),
                crate::tokens::BODY_SIZE,
                crate::tokens::BODY_WEIGHT,
                quiet(),
                crate::tokens::BODY_TRACK_EM
            )
            .accessible(true)
            .label(notice.clone()),
        );
    }
    column = match face {
        CardFace::EmptyPeers => column.child(empty_peers(on_peer)),
        CardFace::Grid => {
            let shelf = element_ref();
            remember_shelf(&shelf);
            let saved = shelf_scroll();
            column.child(
                scroll()
                    .element_ref(&shelf)
                    .flex_1()
                    .flex_shrink(1.0)
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .w_full()
                    .flex()
                    .flex_col()
                    .scrollable(true)
                    .content_offset(point(px(0.0), px(saved)))
                    .on_scroll(|scroll: &ScrollEvent| note_shelf_offset(scroll.offset.y.0))
                    .child(book_grid(library, content_w, on_open)),
            )
        }
        CardFace::Placeholder | CardFace::Page => column,
    };
    body.child(column).into_element()
}

fn card_toolbar() -> AnyElement {
    view()
        .h(px(crate::tokens::TOOLBAR_H))
        .px(px(crate::tokens::TOOLBAR_PX))
        .gap(px(crate::tokens::TOOLBAR_GAP))
        .flex()
        .flex_row()
        .items_center()
        .child(view().flex_1().min_w(px(0.0)))
        .child(sort_control())
        .child(view_toggle())
        .into_element()
}

fn sort_control() -> AnyElement {
    let mut control = view()
        .h(px(crate::tokens::TOOL_H))
        .px(px(crate::tokens::TOOL_PX))
        .gap(px(crate::tokens::TOOL_GAP))
        .rounded(px(crate::tokens::TOOL_RADIUS))
        .flex()
        .flex_shrink(1.0)
        .min_w(px(0.0))
        .flex_row()
        .items_center()
        .accessible(true)
        .label("最近阅读");
    if let Some(path) = crate::icons::glyph_ink("sort-desc", "6E6E73") {
        control = control.child(image(path).size(px(crate::tokens::SORT_ICON)));
    }
    control = control.child(view().flex_1().min_w(px(0.0)).child(fit_text(
        "最近阅读".to_string(),
        crate::tokens::TOOL_SIZE,
        crate::tokens::BODY_WEIGHT,
        icon_ink(),
        crate::tokens::BODY_TRACK_EM,
    )));
    if let Some(path) = crate::icons::glyph_ink("chevron-down", "6E6E73") {
        control = control.child(image(path).size(px(crate::tokens::CHEVRON_ICON)));
    }
    control.into_element()
}

fn view_toggle() -> AnyElement {
    view()
        .p(px(crate::tokens::SEG_PAD))
        .rounded(px(crate::tokens::SEG_RADIUS))
        .bg(wash(crate::tokens::SEG_ALPHA))
        .flex()
        .flex_row()
        .child(view_button("grid", "网格", true))
        .child(view_button("list", "列表", false))
        .into_element()
}

fn view_button(name: &str, label: &str, on: bool) -> AnyElement {
    let mut button = view()
        .w(px(crate::tokens::SEG_BTN_W))
        .h(px(crate::tokens::SEG_BTN_H))
        .rounded(px(crate::tokens::SEG_BTN_RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .accessible(true)
        .label(label.to_string());
    if on {
        button = button
            .bg(rgb(crate::tokens::CARD).to_hsla())
            .shadow(crate::tokens::seg_shadows());
    }
    let hex = if on { "1D1D1F" } else { "6E6E73" };
    if let Some(path) = crate::icons::glyph_ink(name, hex) {
        button = button.child(image(path).size(px(crate::tokens::VIEW_ICON)));
    }
    button.into_element()
}

fn card_header(title: &str, count: u32) -> AnyElement {
    view()
        .h(px(crate::tokens::TITLE_ROW_H))
        .flex()
        .flex_row()
        .items_baseline()
        .gap(px(crate::tokens::TITLE_GAP))
        .child(view().flex_1().min_w(px(0.0)).child({
            // No tooltip. The slot is the width of the card, and a hint of
            // this same string sits centered in the toolbar.
            remember_font(title, crate::tokens::H1_SIZE, crate::tokens::H1_WEIGHT);
            line!(
                title.to_string(),
                crate::tokens::H1_SIZE,
                crate::tokens::H1_WEIGHT,
                ink(),
                crate::tokens::H1_TRACK_EM
            )
            .truncate()
            .accessible(true)
            .label(title.to_string())
        }))
        .child({
            let count_label = format!("{count} 本");
            remember_font(
                &count_label,
                crate::tokens::COUNT_SIZE,
                crate::tokens::COUNT_WEIGHT,
            );
            line!(
                count_label,
                crate::tokens::COUNT_SIZE,
                crate::tokens::COUNT_WEIGHT,
                quiet(),
                0.0
            )
        })
        .into_element()
}

fn book_grid(
    library: &LibraryIn,
    width: f32,
    on_open: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    let mut column = view().flex().flex_col().w_full();
    if !library.continue_reading.is_empty() {
        column = column
            .child(section_label("继续阅读", None))
            .child(continue_row(library, width, on_open.clone()));
    }
    let note = if matches!(library.selected, SelectId::AllBooks) {
        Some("按最近阅读")
    } else {
        None
    };
    column = column.child(section_label("书架", note));
    let grid = spike_library::shelf_grid(width);
    write_geometry(
        &grid,
        library.covers.last().map(|card| card.title.as_str()),
        width,
    );
    let columns = grid.columns.max(1);
    let gap = grid.column_gap;
    let row_w = spike_library::row_width(grid);
    let mut row = view()
        .w(px(row_w))
        .mb(px(grid.row_gap))
        .flex()
        .flex_row()
        .gap(px(gap));
    let mut placed = 0u32;
    for card in &library.covers {
        if placed == columns {
            column = column.child(row);
            row = view()
                .w(px(row_w))
                .mb(px(grid.row_gap))
                .flex()
                .flex_row()
                .gap(px(gap));
            placed = 0;
        }
        row = row.child(cover_tile(card, on_open.clone(), grid));
        placed += 1;
    }
    if placed > 0 {
        column = column.child(row);
    }
    column.into_element()
}

fn continue_row(
    library: &LibraryIn,
    width: f32,
    on_open: impl Fn(String) + Clone + 'static,
) -> AnyElement {
    let layout = crate::tokens::continue_layout(width, library.continue_reading.len() as u32);
    let mut row = view().w(px(width)).flex().flex_row().gap(px(layout.gap));
    for card in library.continue_reading.iter().take(layout.cards as usize) {
        row = row.child(continue_card(card, layout, on_open.clone()));
    }
    row.into_element()
}

fn section_label(label: &str, note: Option<&str>) -> AnyElement {
    let mut row = view()
        .mt(px(crate::tokens::SECTION_MT))
        .mb(px(crate::tokens::SECTION_MB))
        .gap(px(crate::tokens::SECTION_GAP))
        .flex()
        .flex_row()
        .items_baseline()
        .child({
            remember_font(
                label,
                crate::tokens::SECTION_SIZE,
                crate::tokens::SECTION_WEIGHT,
            );
            line!(
                label.to_string(),
                crate::tokens::SECTION_SIZE,
                crate::tokens::SECTION_WEIGHT,
                ink(),
                crate::tokens::BODY_TRACK_EM
            )
        });
    if let Some(note) = note {
        row = row.child(line!(
            note.to_string(),
            crate::tokens::SECTION_NOTE_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            0.0
        ));
    }
    row.into_element()
}

fn continue_card(
    card: &CoverCard,
    layout: crate::tokens::ContinueLayout,
    on_open: impl Fn(String) + 'static,
) -> AnyElement {
    let path = card.open_path.clone();
    let mut tile = view()
        .w(px(layout.card_w))
        .flex_shrink(0.0)
        .rounded(px(crate::tokens::CONTINUE_RADIUS))
        .p(px(crate::tokens::CONTINUE_PAD))
        .gap(px(crate::tokens::CONTINUE_INNER_GAP))
        .bg(rgb(crate::tokens::CONTINUE_BG).to_hsla())
        .flex()
        .flex_row()
        .accessible(true)
        .label(format!("继续阅读 {}", card.title));
    if let Some(path) = path {
        tile = tile.on_press(move || on_open(path.clone()));
    }
    tile.child(cover_art(card, layout.cover_w, layout.cover_h))
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .pt(px(crate::tokens::CONTINUE_TEXT_PT))
                .child(fit_text(
                    card.title.clone(),
                    crate::tokens::CONTINUE_TITLE_SIZE,
                    crate::tokens::CONTINUE_TITLE_WEIGHT,
                    ink(),
                    crate::tokens::BODY_TRACK_EM,
                ))
                .child(meta_line(card, true))
                .child(view().flex_1())
                .child(if card.percent.is_some() {
                    progress_line(card)
                } else {
                    view().into_element()
                })
                .child(fit_text(
                    card.opened.clone(),
                    crate::tokens::TIME_SIZE,
                    crate::tokens::BODY_WEIGHT,
                    faint(),
                    crate::tokens::BODY_TRACK_EM,
                )),
        )
        .into_element()
}

fn progress_line(card: &CoverCard) -> AnyElement {
    let percent = card.percent.unwrap_or(0);
    let mut bar = view()
        .flex_1()
        .h(px(crate::tokens::BAR_H))
        .rounded(px(crate::tokens::BAR_RADIUS))
        .overflow_hidden()
        .bg(wash(crate::tokens::BAR_TRACK_ALPHA))
        .flex()
        .flex_row();
    if percent > 0 {
        bar = bar.child(
            view()
                .h(px(crate::tokens::BAR_H))
                .flex_grow(f32::from(percent))
                .bg(ink()),
        );
        bar = bar.child(view().flex_grow(f32::from(100 - percent)));
    }
    view()
        .gap(px(crate::tokens::BAR_GAP))
        .flex()
        .flex_row()
        .items_center()
        .child(bar)
        .child(line!(
            card.percent
                .map(|value| format!("{value}%"))
                .unwrap_or_default(),
            crate::tokens::PROGRESS_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM
        ))
        .into_element()
}

fn cover_tile(
    card: &CoverCard,
    on_open: impl Fn(String) + 'static,
    grid: spike_library::ShelfGrid,
) -> AnyElement {
    let path = card.open_path.clone();
    let mut tile = view()
        .w(px(grid.cover_w))
        .flex_shrink(0.0)
        .flex()
        .flex_col()
        .accessible(true)
        .label(card.title.clone());
    if let Some(path) = path {
        tile = tile.on_press(move || on_open(path.clone()));
    }
    tile = tile
        .child(cover_art(card, grid.cover_w, grid.cover_h))
        .child(
            view()
                .h(px(grid.caption_h))
                .w(px(grid.cover_w))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    view().mt(px(crate::tokens::SHELF_TITLE_MT)).child(
                        line!(
                            card.title.clone(),
                            crate::tokens::SHELF_TITLE_SIZE,
                            crate::tokens::SHELF_TITLE_WEIGHT,
                            ink(),
                            crate::tokens::BODY_TRACK_EM
                        )
                        .line_height(px(crate::tokens::SHELF_TITLE_SIZE))
                        .truncate()
                        .accessible(true)
                        .label(card.title.clone())
                        .tooltip(
                            text(card.title.clone())
                                .text_size(px(crate::tokens::SHELF_TITLE_SIZE))
                                .text_color(ink()),
                        ),
                    ),
                )
                .child(author_line(card)),
        );
    if let Some(label) = &card.unavailable {
        tile = tile.child(line!(
            label.clone(),
            crate::tokens::SHELF_META_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM
        ));
    }
    tile.into_element()
}

fn author_line(card: &CoverCard) -> AnyElement {
    let mut row = view()
        .mt(px(crate::tokens::SHELF_META_MT))
        .gap(px(crate::tokens::SHELF_META_GAP))
        .flex()
        .flex_row()
        .items_center()
        .child(view().flex_1().min_w(px(0.0)).child(fit_text(
            if card.author.is_empty() {
                " ".to_string()
            } else {
                card.author.clone()
            },
            crate::tokens::SHELF_META_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM,
        )));
    if card.pdf {
        row = row.child(pdf_tag(false));
    }
    if let Some(percent) = card.percent {
        row = row.child(line!(
            format!("{percent}%"),
            crate::tokens::PROGRESS_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM
        ));
    }
    row.into_element()
}

fn meta_line(card: &CoverCard, small_tag: bool) -> AnyElement {
    let mut row = view()
        .mt(px(crate::tokens::CONTINUE_META_MT))
        .gap(px(crate::tokens::SHELF_META_GAP))
        .flex()
        .flex_row()
        .items_center()
        .child(view().flex_1().min_w(px(0.0)).child(fit_text(
            card.position.clone(),
            crate::tokens::CONTINUE_META_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM,
        )));
    if card.pdf {
        row = row.child(pdf_tag(small_tag));
    }
    row.into_element()
}

fn pdf_tag(small: bool) -> AnyElement {
    let size = if small {
        crate::tokens::TAG_SM_SIZE
    } else {
        crate::tokens::TAG_SIZE
    };
    view()
        .rounded(px(crate::tokens::TAG_RADIUS))
        .px(px(crate::tokens::TAG_PX))
        .border(px(crate::tokens::TAG_BORDER))
        .border_color(wash(crate::tokens::TAG_BORDER_ALPHA))
        .child(line!(
            "PDF",
            size,
            crate::tokens::TAG_WEIGHT,
            icon_ink(),
            crate::tokens::TAG_TRACK_EM
        ))
        .into_element()
}

fn cover_art(card: &CoverCard, width: f32, height: f32) -> AnyElement {
    let color = painted_plate_color(card);
    let photo = card.cover.is_some() && card.plate_rgb.is_none() && !card.pdf;
    // A generated plate is a flat token. Its PNG is the same color plus a
    // spine, and an in-flow image of the cover's full height pushes the type
    // below the clip. A photo or a PDF page is the image itself.
    let draw_image = card.cover.is_some() && card.plate_rgb.is_none();
    let (title_ink, author_ink) = plate_inks(color);
    let mut art = view()
        .w(px(width))
        .h(px(height))
        .rounded(px(crate::tokens::SHELF_RADIUS))
        .overflow_hidden()
        .shadow(crate::tokens::cover_shadows())
        .bg(rgb(color).to_hsla());
    if let Some(path) = &card.cover {
        if draw_image {
            art = art.child(
                view()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w(px(width))
                    .h(px(height))
                    .child(
                        image(format!("file://{}", path.display()))
                            .w(px(width))
                            .h(px(height)),
                    ),
            );
        }
    }
    if !draw_image {
        art = art.child(
            view()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .w(px((width * 0.08).max(3.0)))
                .h(px(height))
                .bg(hsla(0.0, 0.0, 0.0, 0.18)),
        );
    }
    if !photo {
        let (title_size, author_size) = plate_type_size(height);
        let text_w = (width - crate::tokens::COVER_LABEL_PX * 2.0).max(0.0);
        art = art.child(
            view()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .w(px(width))
                .h(px(height))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(crate::tokens::COVER_LABEL_PX))
                .gap(px(crate::tokens::SHELF_META_GAP))
                .child(plate_line(
                    card.title.clone(),
                    text_w,
                    title_size,
                    crate::tokens::H1_WEIGHT,
                    title_ink,
                    0.0,
                ))
                .child(plate_line(
                    card.author.clone(),
                    text_w,
                    author_size,
                    crate::tokens::BODY_WEIGHT,
                    author_ink,
                    crate::tokens::LABEL_TRACK_EM,
                )),
        );
    }
    if height >= 100.0 {
        art = art.child(
            view()
                .absolute()
                .right(px(0.0))
                .top(px(8.0))
                .w(px(6.0))
                .h(px((height - 16.0).max(8.0)))
                .bg(hsla(0.0, 0.0, 0.0, 0.28)),
        );
    }
    art.into_element()
}

fn plate_line(
    label: String,
    width: f32,
    size: f32,
    weight: FontWeight,
    color: Hsla,
    tracking: f32,
) -> AnyElement {
    remember_font(&label, size, weight);
    let line_h = plate_line_box(size);
    view()
        .w(px(width))
        .h(px(line_h))
        .min_w(px(0.0))
        .flex_shrink(0.0)
        .child(
            line!(label.clone(), size, weight, color, tracking)
                .w(px(width))
                .h(px(line_h))
                .text_center()
                .truncate()
                .accessible(true)
                .label(label),
        )
        .into_element()
}

fn face_color(plate: Option<u32>, sampled: Option<u32>, title: &str) -> u32 {
    plate.or(sampled).unwrap_or_else(|| fallback_color(title))
}

fn painted_plate_color(card: &CoverCard) -> u32 {
    face_color(card.plate_rgb, sample_cover(card), &card.title)
}

fn sample_cover(card: &CoverCard) -> Option<u32> {
    if !card.pdf {
        return None;
    }
    let path = card.cover.as_ref()?;
    let bytes = std::fs::read(path).ok()?;
    spike_library::png_center_rgb(&bytes)
}

fn plate_inks(color: u32) -> (Hsla, Hsla) {
    let red = ((color >> 16) & 0xff) as f32;
    let green = ((color >> 8) & 0xff) as f32;
    let blue = (color & 0xff) as f32;
    let luma = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
    if luma > 170.0 {
        (ink(), quiet())
    } else {
        (
            rgb(crate::tokens::CARD).to_hsla(),
            hsla(0.0, 0.0, 1.0, 0.82),
        )
    }
}

fn write_geometry(grid: &spike_library::ShelfGrid, last_title: Option<&str>, content_w: f32) {
    let body = serde_json::json!({
        "cover_w": grid.cover_w,
        "cover_h": grid.cover_h,
        "column_gap": grid.column_gap,
        "row_gap": grid.row_gap,
        "columns": grid.columns,
        "caption_h": grid.caption_h,
        "scrolls": grid.scrolls,
        "last_title": last_title.unwrap_or(""),
        "content_w": content_w,
        "chrome_center": crate::tokens::chrome_center(),
        "inset": INSET,
        "card_pad_x": crate::tokens::CARD_PAD_X,
    });
    let dir = "/tmp/superreader-spike";
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(format!("{dir}/geometry.json"), body.to_string());
}

fn fallback_color(title: &str) -> u32 {
    let mut hash = 0u32;
    for byte in title.as_bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u32::from(*byte));
    }
    const COLORS: [u32; 5] = [
        0x002F_4F3E,
        0x008B_3A3A,
        0x001E_3A5F,
        0x008A_5A2B,
        0x003D_4C5A,
    ];
    COLORS[(hash as usize) % COLORS.len()]
}

#[derive(Clone, Copy)]
pub enum Peer {
    AppleBooks,
    Calibre,
    AddFolder,
}

fn empty_peers(on_peer: impl Fn(Peer) + Clone + 'static) -> AnyElement {
    view()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(22.0))
        .child(
            text("从这些来源加入书籍")
                .text_size(px(13.0))
                .text_color(quiet()),
        )
        .child(
            view()
                .w_full()
                .flex()
                .flex_row()
                .flex_wrap()
                .justify_center()
                .gap(px(16.0))
                .child(peer_tile(
                    "Apple Books",
                    "book",
                    Peer::AppleBooks,
                    on_peer.clone(),
                ))
                .child(peer_tile(
                    "calibre",
                    "library",
                    Peer::Calibre,
                    on_peer.clone(),
                ))
                .child(peer_tile("添加文件夹", "folder", Peer::AddFolder, on_peer)),
        )
        .into_element()
}

fn peer_tile(label: &str, icon: &str, peer: Peer, on_peer: impl Fn(Peer) + 'static) -> AnyElement {
    view()
        .w(px(148.0))
        .h(px(112.0))
        .rounded(px(12.0))
        .bg(rgb(0x00F6_F6F8).to_hsla())
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .accessible(true)
        .label(label.to_string())
        .on_press(move || on_peer(peer))
        .child(glyph_box(
            icon,
            false,
            crate::tokens::ROW_ICON,
            Some("6E6E73"),
        ))
        .child(
            text(label.to_string())
                .text_size(px(13.0))
                .text_color(ink()),
        )
        .into_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> Window {
        Window {
            width: 1440.0,
            height: 900.0,
        }
    }

    #[test]
    fn plate_ink_uses_the_marker_or_the_sampled_page_before_the_title_hash() {
        let title = "The Art of Computer Programming";
        assert_eq!(fallback_color(title), 0x002F_4F3E);
        assert_eq!(face_color(Some(0x7D8C6A), Some(0xEDE6D5), title), 0x7D8C6A);
        assert_eq!(face_color(None, Some(0xEDE6D5), title), 0xEDE6D5);
        assert_eq!(face_color(None, None, title), 0x2F4F3E);
    }

    #[test]
    fn a_wheel_on_the_shelf_is_the_shelf_and_a_wheel_on_the_chat_is_not() {
        note_shelf_hit([266.0, 180.0, 808.0, 600.0]);
        assert!(shelf_hit_contains(700.0, 613.0));
        assert!(!shelf_hit_contains(1200.0, 613.0));
        assert!(!shelf_hit_contains(700.0, 40.0));
    }

    fn empty_library() -> LibraryIn {
        LibraryIn {
            empty: true,
            book_count: 0,
            counts: LibraryCounts::default(),
            sources: Vec::new(),
            selected: SelectId::AllBooks,
            continue_reading: Vec::new(),
            covers: Vec::new(),
            lists: Vec::new(),
            chat: Vec::new(),
            notice: None,
        }
    }

    fn near(got: f64, want: f64) {
        assert!((got - want).abs() < 0.01, "{got} vs {want}");
    }

    #[test]
    fn launch_is_full_with_both_columns() {
        let form = launch();
        let placed = place(
            &PlaceIn {
                form,
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 92.0,
                    top: 0.0,
                },
            },
            &empty_library(),
        );
        let sidebar = placed.sidebar.expect("sidebar");
        near(sidebar.w, 220.0);
        let chat = placed.chat.expect("chat");
        near(chat.w, 320.0);
        let CardBox::Placed {
            rect,
            radius,
            shadow,
        } = placed.card
        else {
            panic!("card");
        };
        near(rect.x, 226.0);
        near(rect.y, 6.0);
        near(rect.w, 1440.0 - 220.0 - 320.0 - 12.0);
        near(rect.h, 900.0 - 12.0);
        near(radius, 12.0);
        assert!(shadow);
        assert_eq!(placed.face, CardFace::EmptyPeers);
        assert_eq!(placed.groups, ["书库", "笔记", "书单", "来源"]);
    }

    #[test]
    fn a_narrow_toggle_keeps_a_shown_chat_preference() {
        let shown = launch();
        assert_eq!(panel_command(shown, true), shown);
        let hidden = panel_command(shown, false);
        assert!(matches!(
            hidden,
            Form::Full {
                chat: Presence::Hidden,
                ..
            }
        ));
        assert_eq!(panel_command(hidden, true), shown);
    }

    #[test]
    fn a_minimum_window_hides_chat_and_keeps_two_columns() {
        let library = empty_library();
        let placed = place(
            &PlaceIn {
                form: launch(),
                window: Window {
                    width: crate::tokens::MIN_WINDOW_W,
                    height: crate::tokens::library_min_height(),
                },
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 78.0,
                    top: 0.0,
                },
            },
            &library,
        );
        assert!(placed.chat.is_none());
        let CardBox::Placed { rect, .. } = placed.card else {
            panic!("card");
        };
        assert!((rect.x - (220.0 + INSET)).abs() < 0.5);
        let content = crate::tokens::content_width(rect.w as f32, 0.0, 0.0);
        let grid = spike_library::shelf_grid(content);
        assert_eq!(grid.columns, 2);
        assert!((grid.cover_w - 112.0).abs() < 0.5);
        let nodes = layout_nodes(&placed, &library);
        assert!(nodes.iter().any(|node| node["id"] == "shelf.row"));
        for node in &nodes {
            let frame = node["frame"].as_array().expect("frame");
            let container = node["container"].as_array().expect("container");
            let x = frame[0].as_f64().unwrap();
            let width = frame[2].as_f64().unwrap();
            let cx = container[0].as_f64().unwrap();
            let cw = container[2].as_f64().unwrap();
            assert!(
                x + 0.5 >= cx && x + width <= cx + cw + 0.5,
                "{node} leaves its container"
            );
            assert!(
                x + width <= crate::tokens::MIN_WINDOW_W + 0.5,
                "{node} leaves the window"
            );
        }
        let settings = nodes.iter().find(|node| node["id"] == "sidebar.settings");
        let reading = nodes.iter().find(|node| node["label"] == "正在读");
        let settings = settings.expect("settings row");
        let reading = reading.expect("正在读");
        assert!(
            !frames_overlap(
                settings["frame"].as_array().unwrap(),
                reading["frame"].as_array().unwrap()
            ),
            "正在读 overlaps 设置"
        );
        assert!((rect.h - crate::tokens::library_stack() as f64).abs() < 0.5);
    }

    fn frames_overlap(a: &[serde_json::Value], b: &[serde_json::Value]) -> bool {
        let box_of = |items: &[serde_json::Value]| {
            (
                items[0].as_f64().unwrap_or(0.0),
                items[1].as_f64().unwrap_or(0.0),
                items[2].as_f64().unwrap_or(0.0),
                items[3].as_f64().unwrap_or(0.0),
            )
        };
        let (ax, ay, aw, ah) = box_of(a);
        let (bx, by, bw, bh) = box_of(b);
        ax < bx + bw - 0.5 && bx < ax + aw - 0.5 && ay < by + bh - 0.5 && by < ay + ah - 0.5
    }

    #[test]
    fn hiding_a_column_keeps_the_six_point_inset() {
        let hidden_side = step(launch(), Command::ToggleSidebar);
        let placed = place(
            &PlaceIn {
                form: hidden_side,
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 92.0,
                    top: 0.0,
                },
            },
            &empty_library(),
        );
        assert!(placed.sidebar.is_none());
        let CardBox::Placed { rect, .. } = placed.card else {
            panic!("card");
        };
        near(rect.x, 6.0);
        near(rect.w, 1440.0 - 320.0 - 12.0);
        let hidden_chat = step(launch(), Command::TogglePanel);
        let placed = place(
            &PlaceIn {
                form: hidden_chat,
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 92.0,
                    top: 0.0,
                },
            },
            &empty_library(),
        );
        assert!(placed.chat.is_none());
        let CardBox::Placed { rect, .. } = placed.card else {
            panic!("card");
        };
        near(rect.x, 226.0);
        near(rect.w, 1440.0 - 220.0 - 12.0);
        let right = window().width - (rect.x + rect.w);
        near(right, 6.0);
    }

    #[test]
    fn a_toggle_twice_restores_full_and_does_not_leave_it() {
        let once = step(launch(), Command::TogglePanel);
        assert!(matches!(
            once,
            Form::Full {
                chat: Presence::Hidden,
                ..
            }
        ));
        assert_eq!(step(once, Command::TogglePanel), launch());
        assert_eq!(
            step(Form::Immersive, Command::ToggleSidebar),
            Form::Immersive
        );
    }

    #[test]
    fn a_shown_margin_has_no_chat_width() {
        let form = Form::PageAndPanel {
            sidebar: Presence::Hidden,
            margin: Presence::Shown,
        };
        let placed = place(
            &PlaceIn {
                form,
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: false,
                    bar: 0.0,
                    top: 0.0,
                },
            },
            &empty_library(),
        );
        assert!(placed.chat.is_none());
        let CardBox::Placed { rect, .. } = placed.card;
        assert!((rect.w - (1440.0 - INSET * 2.0 - MARGIN_W)).abs() < 0.1);
    }

    #[test]
    fn the_margin_keeps_its_state_across_818_and_the_narrow_page_stays() {
        let hidden = reading_frame(600.0, 800.0, 0.0, 0.0, false);
        let shown = reading_frame(600.0, 800.0, 0.0, 0.0, true);
        assert_eq!(hidden.card, shown.card);
        assert!(hidden.margin.is_none());
        assert!(shown.covers);
        assert!(shown.margin.is_some());
        let below = reading_frame(817.0, 800.0, 0.0, 0.0, true);
        let above = reading_frame(818.0, 800.0, 0.0, 0.0, true);
        assert!(below.covers);
        assert!(!above.covers);
        assert_eq!(
            below.card,
            reading_frame(817.0, 800.0, 0.0, 0.0, false).card
        );
        let above_hidden = reading_frame(818.0, 800.0, 0.0, 0.0, false);
        assert!((above_hidden.card.w - above.card.w - MARGIN_W).abs() < 0.1);
        let wide_hidden = reading_frame(1440.0, 900.0, 0.0, 0.0, false);
        let wide_shown = reading_frame(1440.0, 900.0, 0.0, 0.0, true);
        assert!(wide_hidden.margin.is_none());
        assert!((wide_hidden.card.w - wide_shown.card.w - MARGIN_W).abs() < 0.1);
        assert!(!margin_is_column(817.0));
        assert!(margin_is_column(818.0));
        let wide = page_frame(1440.0, 900.0, true, false);
        let narrow = page_frame(817.0, 800.0, true, false);
        assert_eq!(wide, page_frame(1440.0, 900.0, true, false));
        assert_eq!(narrow, page_frame(817.0, 800.0, true, false));
        assert!(
            (wide.card.w - page_frame(1440.0, 900.0, false, false).card.w + MARGIN_W).abs() < 0.1
        );
        assert_eq!(narrow.card, page_frame(817.0, 800.0, false, false).card);
    }

    #[test]
    fn a_column_mode_does_not_move_the_page_frame() {
        let wide = page_frame(1440.0, 900.0, true, false);
        let narrow = page_frame(578.0, 528.0, true, false);
        let immersive = page_frame(1440.0, 900.0, true, true);
        assert_eq!(wide, page_frame(1440.0, 900.0, true, false));
        assert_eq!(narrow.card, page_frame(578.0, 528.0, false, false).card);
        assert!(narrow.covers);
        assert!(immersive.margin.is_none());
        assert_eq!(immersive, page_frame(1440.0, 900.0, false, true));
    }

    #[test]
    fn a_covered_page_turns_on_the_visible_edge() {
        let frame = reading_frame(578.0, 900.0, 0.0, 0.0, true);
        assert!(frame.covers);
        let (x, y, w, h) = spike_report::fitted_pdf_page(
            frame.card.x,
            frame.card.y,
            frame.card.w,
            frame.card.h,
            612.0,
            792.0,
        );
        let (visible_x, visible_w) = visible_page_span(x, w, &frame);
        let margin = frame.margin.expect("cover");
        assert!(visible_w < w);
        let covered = x + w - w * crate::visit::PAGE_EDGE / 2.0;
        let mid_y = y + h / 2.0;
        assert!(covered >= margin.x);
        assert_eq!(
            crate::visit::page_edge(covered, mid_y, x, y, w, h),
            crate::visit::PageEdge::Forward
        );
        assert_eq!(
            crate::visit::page_edge(covered, mid_y, visible_x, y, visible_w, h),
            crate::visit::PageEdge::None
        );
        let forward = visible_x + visible_w - 1.0;
        assert!(forward < margin.x);
        assert_eq!(
            crate::visit::page_edge(forward, mid_y, visible_x, y, visible_w, h),
            crate::visit::PageEdge::Forward
        );
        assert_eq!(
            crate::visit::page_edge(visible_x + 1.0, mid_y, visible_x, y, visible_w, h),
            crate::visit::PageEdge::Back
        );
        assert_eq!(
            crate::visit::page_edge(
                visible_x + visible_w / 2.0,
                mid_y,
                visible_x,
                y,
                visible_w,
                h
            ),
            crate::visit::PageEdge::None
        );

        let column = reading_frame(818.0, 900.0, 0.0, 0.0, true);
        assert!(!column.covers);
        let (cx, cy, cw, ch) = spike_report::fitted_pdf_page(
            column.card.x,
            column.card.y,
            column.card.w,
            column.card.h,
            612.0,
            792.0,
        );
        assert_eq!(visible_page_span(cx, cw, &column), (cx, cw));
        let column_margin = column.margin.expect("column");
        let right = cx + cw - 1.0;
        assert!(right < column_margin.x);
        assert_eq!(
            crate::visit::page_edge(right, cy + ch / 2.0, cx, cy, cw, ch),
            crate::visit::PageEdge::Forward
        );
    }

    #[test]
    fn a_connected_source_keeps_its_count_and_a_denial_drops_it() {
        let mut rows = Vec::new();
        remember_source(&mut rows, "Apple Books", 4, true);
        remember_source(&mut rows, "calibre", 6, true);
        assert_eq!(rows[0].count, 4);
        remember_source(&mut rows, "Apple Books", 5, true);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].count, 5);
        assert_eq!(rows[1].label, "calibre");
        assert_eq!(rows[1].count, 6);
        remember_source(&mut rows, "calibre", 0, false);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Apple Books");
    }

    #[test]
    fn books_fill_the_grid() {
        let mut library = empty_library();
        library.empty = false;
        library.book_count = 4;
        let placed = place(
            &PlaceIn {
                form: launch(),
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: false,
                    bar: 0.0,
                    top: 0.0,
                },
            },
            &library,
        );
        assert_eq!(placed.face, CardFace::Grid);
    }

    fn frame_of(nodes: &[serde_json::Value], id: &str) -> [f32; 4] {
        let node = nodes
            .iter()
            .find(|node| node["id"] == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        let frame = node["frame"].as_array().expect("frame");
        [
            frame[0].as_f64().unwrap() as f32,
            frame[1].as_f64().unwrap() as f32,
            frame[2].as_f64().unwrap() as f32,
            frame[3].as_f64().unwrap() as f32,
        ]
    }

    fn assert_shared_centers(width: f64, height: f64) {
        let library = empty_library();
        let placed = place(
            &PlaceIn {
                form: launch(),
                window: Window { width, height },
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 92.0,
                    top: 0.0,
                },
            },
            &library,
        );
        let nodes = layout_nodes(&placed, &library);
        let center = crate::tokens::chrome_center();
        let mut ids = vec!["sidebar.toggle", "toolbar", "toolbar.sort", "toolbar.view"];
        if placed.chat.is_some() {
            ids.extend([
                "chat.head",
                "chat.avatar",
                "chat.name",
                "chat.new",
                "chat.panel",
            ]);
        }
        for id in ids {
            let frame = frame_of(&nodes, id);
            let mid = frame[1] + frame[3] / 2.0;
            assert!((mid - center).abs() < 0.5, "{id} at {width} is {mid}");
        }
        let toggle = frame_of(&nodes, "sidebar.toggle");
        assert!((toggle[0] - 92.0).abs() < 0.01);
        if placed.chat.is_some() {
            let name = frame_of(&nodes, "chat.name");
            let sub = frame_of(&nodes, "chat.sub");
            assert!((name[1] + name[3] - (sub[1] + sub[3])).abs() < 0.01);
            let avatar = frame_of(&nodes, "chat.avatar");
            let chat = frame_of(&nodes, "chat");
            assert!((avatar[0] - (chat[0] + crate::tokens::CHAT_PAD_X)).abs() < 0.01);
            let composer = frame_of(&nodes, "composer");
            let placeholder = frame_of(&nodes, "composer.placeholder");
            let attach = frame_of(&nodes, "composer.attach");
            let send = frame_of(&nodes, "composer.send");
            let window_h = height as f32;
            let window_w = width as f32;
            let bottom = window_h - (composer[1] + composer[3]);
            let right = window_w - (composer[0] + composer[2]);
            assert!(
                (bottom - crate::tokens::WINDOW_CORNER).abs() < 0.5,
                "{bottom}"
            );
            assert!(
                (right - crate::tokens::WINDOW_CORNER).abs() < 0.5,
                "{right}"
            );
            assert!((placeholder[0] - attach[0]).abs() < 0.01);
            assert!((placeholder[0] - (composer[0] + crate::tokens::COMPOSER_PAD)).abs() < 0.01);
            assert!(
                ((composer[0] + composer[2]) - (send[0] + send[2]) - crate::tokens::COMPOSER_PAD)
                    .abs()
                    < 0.01
            );
            assert!((attach[1] + attach[3] / 2.0 - (send[1] + send[3] / 2.0)).abs() < 0.01);
            assert!((attach[3] - send[3]).abs() < 0.01);
        }
        let settings = frame_of(&nodes, "sidebar.settings");
        let settings_gap = height as f32 - (settings[1] + settings[3]);
        assert!(
            (settings_gap - crate::tokens::WINDOW_CORNER).abs() < 0.5,
            "{settings_gap}"
        );
        let search = frame_of(&nodes, "sidebar.search");
        assert!((search[0] - crate::tokens::SIDE_PAD_X).abs() < 0.01);
        let row = frame_of(&nodes, "sidebar.row.0");
        let icon = frame_of(&nodes, "sidebar.row.0.icon");
        let label = frame_of(&nodes, "sidebar.row.0.label");
        assert!((row[0] - crate::tokens::SIDE_PAD_X).abs() < 0.01);
        let row_mid = row[1] + row[3] / 2.0;
        assert!((icon[1] + icon[3] / 2.0 - row_mid).abs() < 0.5);
        assert!((label[1] + label[3] / 2.0 - row_mid).abs() < 0.5);
        let title = frame_of(&nodes, "shelf.header.title");
        let count = frame_of(&nodes, "shelf.header.count");
        assert!((title[1] + title[3] - (count[1] + count[3])).abs() < 0.01);
        let header = frame_of(&nodes, "shelf.header");
        let section = frame_of(&nodes, "shelf.section");
        assert!((header[0] - section[0]).abs() < 0.01);
        let card = frame_of(&nodes, "card");
        assert!((header[0] - (card[0] + crate::tokens::CARD_PAD_X)).abs() < 0.01);
    }

    #[test]
    fn the_scope_chip_hugs_its_title() {
        let mut library = empty_library();
        library.continue_reading.push(CoverCard {
            title: "Pride and Prejudice".into(),
            author: String::new(),
            cover: None,
            open_path: None,
            pdf: false,
            percent: None,
            page: None,
            position: String::new(),
            opened: String::new(),
            plate: true,
            plate_rgb: None,
            unavailable: None,
        });
        let placed = place(
            &PlaceIn {
                form: launch(),
                window: window(),
                chrome: crate::tokens::ChromeIn {
                    lights: true,
                    bar: 92.0,
                    top: 0.0,
                },
            },
            &library,
        );
        let nodes = layout_nodes(&placed, &library);
        let chip = frame_of(&nodes, "composer.chip");
        let natural = scope_chip_width("Pride and Prejudice");
        assert!((chip[2] - natural).abs() < 0.01, "{}", chip[2]);
        assert!(chip[2] < 180.0, "{}", chip[2]);
        let send = frame_of(&nodes, "composer.send");
        let attach = frame_of(&nodes, "composer.attach");
        assert!((chip[1] + chip[3] / 2.0 - (send[1] + send[3] / 2.0)).abs() < 0.01);
        assert!((chip[3] - send[3]).abs() < 0.01);
        assert!((attach[1] + attach[3] / 2.0 - (chip[1] + chip[3] / 2.0)).abs() < 0.01);
        assert!(chip[0] > attach[0] + attach[2]);
        assert!(chip[0] + chip[2] < send[0]);
    }

    #[test]
    fn hidden_lights_use_the_sidebar_pad() {
        let placed = place(
            &PlaceIn {
                form: launch(),
                window: Window {
                    width: 1512.0,
                    height: 949.0,
                },
                chrome: crate::tokens::ChromeIn {
                    lights: false,
                    bar: 78.0,
                    top: 0.0,
                },
            },
            &empty_library(),
        );
        let nodes = layout_nodes(&placed, &empty_library());
        let toggle = frame_of(&nodes, "sidebar.toggle");
        let search = frame_of(&nodes, "sidebar.search");
        let title = frame_of(&nodes, "shelf.header.title");
        let card = frame_of(&nodes, "card");
        assert!((toggle[0] - crate::tokens::SIDE_PAD_X).abs() < 0.01);
        assert!((toggle[0] - 78.0).abs() > 1.0);
        assert!(search[1] >= toggle[1] + toggle[3] - 0.5);
        assert!(search[1] + search[3] <= 949.0);
        assert!(title[1] >= card[1]);
        assert!(title[1] + title[3] <= card[1] + card[3]);
        let dropped = place(
            &PlaceIn {
                form: launch(),
                window: Window {
                    width: 1512.0,
                    height: 949.0,
                },
                chrome: crate::tokens::ChromeIn {
                    lights: false,
                    bar: 78.0,
                    top: 20.0,
                },
            },
            &empty_library(),
        );
        let lowered = layout_nodes(&dropped, &empty_library());
        let lowered_title = frame_of(&lowered, "shelf.header.title");
        assert!((lowered_title[1] - title[1] - 20.0).abs() < 0.5);
        assert!(lowered_title[1] >= 20.0);
    }

    #[test]
    fn chrome_boxes_share_one_center_at_every_window() {
        assert_shared_centers(1440.0, 900.0);
        assert_shared_centers(
            crate::tokens::MIN_WINDOW_W,
            crate::tokens::library_min_height(),
        );
        assert_shared_centers(1920.0, 1080.0);
    }

    #[test]
    fn a_wheel_on_the_margin_stays_with_the_cards() {
        struct Restore {
            immersive: bool,
            margin: bool,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                crate::ease::settle(self.immersive, self.margin);
            }
        }
        let restore = Restore {
            immersive: immersive_on(),
            margin: margin_shown(),
        };
        crate::ease::settle(false, true);
        let wide = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        let wide_again = crate::ease::present(1440.0, 900.0, 48.0, crate::ease::current());
        assert_eq!(
            wide.margin.map(|rect| rect.x),
            wide_again.margin.map(|rect| rect.x)
        );
        assert!(margin_claims_wheel(1300.0, 200.0, wide.margin));
        assert!(!margin_claims_wheel(400.0, 200.0, wide.margin));
        assert_eq!(
            crate::visit::wheel_step(true, false),
            crate::visit::WheelStep::Margin
        );
        assert_eq!(
            crate::visit::wheel_step(false, false),
            crate::visit::WheelStep::Page
        );
        assert_eq!(
            crate::visit::wheel_step(true, true),
            crate::visit::WheelStep::Shelf
        );
        let narrow = reading_frame(760.0, 900.0, 0.0, 0.0, true);
        assert!(margin_claims_wheel(600.0, 200.0, narrow.margin));
        crate::ease::settle(true, true);
        let immersed = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert!(!margin_claims_wheel(1300.0, 200.0, immersed.margin));
        assert_eq!(
            crate::visit::wheel_step(false, false),
            crate::visit::WheelStep::Page
        );
        crate::ease::settle(false, false);
        let hidden = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert!(!margin_claims_wheel(1300.0, 200.0, hidden.margin));
        drop(restore);
    }

    #[test]
    fn the_margin_keeps_the_departing_page_and_maps_its_line() {
        let shown = crate::visit::margin_pdf_index(10, 4, 3, true).unwrap();
        assert_eq!(crate::visit::margin_page_number(shown), 4);
        let landed_index = crate::visit::margin_pdf_index(10, 4, 3, false).unwrap();
        assert_eq!(crate::visit::margin_page_number(landed_index), 5);
        assert!(crate::visit::margin_pdf_index(0, 0, 0, true).is_none());
        let live = spike_library::margin::LivePage {
            book: "a.pdf".into(),
            spine: String::new(),
            page: crate::visit::margin_page_number(shown),
            page_w: 400.0,
            page_h: 500.0,
            moving: true,
        };
        let hold = spike_library::margin::Hold {
            book: "a.pdf".into(),
            page: 4,
            scroll: 0.0,
            selected: Some(7),
            ..spike_library::margin::Hold::default()
        };
        let page = Rect {
            x: 10.0,
            y: 20.0,
            w: 800.0,
            h: 1000.0,
        };
        let note = spike_library::margin::StoredNote {
            id: 7,
            page: 4,
            spine: String::new(),
            start: 0,
            end: 4,
            passage: "word ".repeat(80),
            body: String::new(),
            author: "读者".into(),
            quads: vec![
                spike_library::margin::Quad {
                    x: 10.0,
                    y: 20.0,
                    w: 30.0,
                    h: 8.0,
                },
                spike_library::margin::Quad {
                    x: 4.0,
                    y: 40.0,
                    w: 12.0,
                    h: 8.0,
                },
            ],
            text_right: 90.0,
            replies: Vec::new(),
        };
        let moving = settle_margin(
            &hold,
            &live,
            false,
            &page,
            1200.0,
            900.0,
            &[note.clone()],
            None,
        );
        assert_eq!(moving.hold.page, 4);
        assert!(moving.hold.scroll.abs() < 0.01);
        assert_eq!(moving.hold.selected, Some(7));
        let refused = settle_margin(
            &spike_library::margin::Hold {
                scroll: 12.0,
                ..moving.hold.clone()
            },
            &live,
            false,
            &page,
            1200.0,
            900.0,
            &[note.clone()],
            None,
        );
        assert!(refused.scene.scroll_max.abs() < 0.01);
        assert!(refused.hold.scroll.abs() < 0.01);
        assert!((refused.scene.cards[0].y - 60.0).abs() < 0.01);
        assert!(!refused.scene.cards[0].segments.is_empty());
        assert!((refused.scene.cards[0].segments[0].x - 190.0).abs() < 0.01);
        let cards = spike_library::margin::cards_from(&[note.clone()], moving.fit);
        assert!((cards[0].line.top - 60.0).abs() < 0.01);
        assert!((cards[0].line.text_right - 190.0).abs() < 0.01);
        assert!(cards[0].line.highlight_right < cards[0].line.text_right);
        assert!(cards[0].hover_h > spike_library::margin::open_height("", &[]));
        let placed = &moving.scene.cards[0];
        assert!((placed.y - 60.0).abs() < 0.01);
        assert!((placed.segments[0].x - 190.0).abs() < 0.01);
        let mut tall_note = note.clone();
        tall_note.passage = "word ".repeat(160);
        let tall = settle_margin(
            &spike_library::margin::Hold {
                scroll: 0.0,
                ..moving.hold.clone()
            },
            &live,
            false,
            &page,
            1200.0,
            900.0,
            &[tall_note.clone()],
            None,
        );
        assert!(tall.scene.scroll_max > 0.5);
        assert!(!tall.scene.cards[0].segments.is_empty());
        let hidden = settle_margin(
            &spike_library::margin::Hold {
                scroll: tall.scene.scroll_max,
                ..tall.hold.clone()
            },
            &live,
            false,
            &page,
            1200.0,
            900.0,
            &[tall_note.clone()],
            None,
        );
        assert!((hidden.hold.scroll - tall.scene.scroll_max).abs() < 0.01);
        assert!(hidden.scene.cards[0].segments.is_empty());
        let restored = settle_margin(
            &spike_library::margin::Hold {
                scroll: 0.0,
                ..hidden.hold.clone()
            },
            &live,
            false,
            &page,
            1200.0,
            900.0,
            &[tall_note],
            None,
        );
        assert!(restored.hold.scroll.abs() < 0.01);
        assert!(!restored.scene.cards[0].segments.is_empty());
        assert!((restored.scene.cards[0].segments[0].x - 190.0).abs() < 0.01);
        let mut arrived = live.clone();
        arrived.moving = false;
        arrived.page = 5;
        let still = settle_margin(
            &moving.hold,
            &arrived,
            true,
            &page,
            1200.0,
            900.0,
            &[],
            None,
        );
        assert_eq!(still.hold.page, 4);
        assert_eq!(still.hold.selected, Some(7));
        let landed = settle_margin(
            &still.hold,
            &arrived,
            false,
            &page,
            1200.0,
            900.0,
            &[],
            None,
        );
        assert_eq!(landed.hold.page, 5);
        assert!(landed.hold.scroll.abs() < 0.01);
        assert!(landed.hold.selected.is_none());
        let other = spike_library::margin::LivePage {
            book: "b.pdf".into(),
            page: 1,
            moving: false,
            ..arrived
        };
        let changed = settle_margin(&landed.hold, &other, false, &page, 1200.0, 900.0, &[], None);
        assert_eq!(changed.hold.book, "b.pdf");
        assert_eq!(changed.hold.page, 1);
        assert!(changed.hold.selected.is_none());
    }

    #[test]
    fn a_broken_annotation_store_keeps_its_error() {
        struct Restore(Option<String>);
        impl Drop for Restore {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
                    None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
                }
            }
        }
        let restore = Restore(std::env::var("SUPERREADER_SUPPORT_DIR").ok());
        let dir = std::env::temp_dir().join(format!("margin-fault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        std::fs::write(dir.join("annotations.sqlite"), b"not a database").unwrap();
        let err =
            load_margin("/books/a.pdf", 2, "").expect_err("a bad file is not an empty margin");
        assert!(!err.is_empty());
        let _ = std::fs::remove_file(dir.join("annotations.sqlite"));
        let notes = spike_library::margin::Notes::open(&dir.join("annotations.sqlite")).unwrap();
        notes
            .insert(&spike_library::margin::NewNote {
                book: "/books/a.pdf".into(),
                page: 2,
                spine: String::new(),
                start: 0,
                end: 3,
                passage: "ink".into(),
                body: "a note".into(),
                author: "读者".into(),
                quads: vec![spike_library::margin::Quad {
                    x: 1.0,
                    y: 2.0,
                    w: 3.0,
                    h: 4.0,
                }],
                text_right: 8.0,
            })
            .unwrap();
        drop(notes);
        let loaded = load_margin("/books/a.pdf", 2, "").unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(load_margin("", 2, "").unwrap().is_empty());
        drop(restore);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn return_saves_the_margin_note_and_keeps_a_failed_draft() {
        struct Restore(Option<String>);
        impl Drop for Restore {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
                    None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
                }
            }
        }
        let restore = Restore(std::env::var("SUPERREADER_SUPPORT_DIR").ok());
        let dir = std::env::temp_dir().join(format!("margin-commit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        std::fs::write(dir.join("annotations.sqlite"), b"not a database").unwrap();
        let passage = spike_library::margin::passage_from(spike_library::margin::Selection {
            book: "/books/a.pdf".into(),
            gen: 2,
            page: 4,
            spine: String::new(),
            start: 0,
            end: 3,
            text: "ink".into(),
            quads: vec![spike_library::margin::Quad {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 8.0,
            }],
            text_right: 70.0,
        })
        .unwrap();
        let live = spike_library::margin::LiveBook {
            book: passage.book.clone(),
            gen: passage.gen,
        };
        let place = spike_library::margin::Visible {
            book: live.clone(),
            page: passage.page,
            spine: passage.spine.clone(),
            moving: false,
        };
        let status = serde_json::json!({
            "book_gen": passage.gen,
            "passage": crate::visit::passage_value(Some(&passage)),
        });
        let field = note_from_status(
            &spike_library::margin::NoteField::default(),
            &status,
            &passage.book,
        );
        assert_eq!(field.passage.as_ref().unwrap().page, 4);
        let failed = commit_margin(
            &spike_library::margin::NoteField {
                body: "别丢".into(),
                ..field.clone()
            },
            "别丢",
            &place,
        );
        assert_eq!(failed.body, "别丢");
        assert!(!failed.notice.is_empty());
        assert!(failed.fingerprint.is_none());
        let _ = std::fs::remove_file(dir.join("annotations.sqlite"));
        let saved = commit_margin(&failed, "别丢", &place);
        assert!(saved.body.is_empty());
        assert!(saved.notice.is_empty());
        let loaded = load_margin(&passage.book, 4, &passage.spine).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].body, "别丢");
        assert_eq!(loaded[0].passage, "ink");
        let cards = spike_library::margin::cards_from(
            &loaded,
            spike_library::margin::Fit {
                origin_x: 0.0,
                origin_y: 0.0,
                scale: 1.0,
            },
        );
        assert_eq!(cards.len(), 1);
        assert!((cards[0].line.top - 20.0).abs() < 0.01);
        assert!((cards[0].line.text_right - 70.0).abs() < 0.01);
        let echo = commit_margin(&saved, "别丢", &place);
        assert!(echo.body.is_empty());
        assert_eq!(
            load_margin(&passage.book, 4, &passage.spine).unwrap().len(),
            1
        );
        assert!(load_margin("/books/b.epub", 4, "").unwrap().is_empty());
        let switched = serde_json::json!({
            "book_gen": passage.gen + 1,
            "passage": serde_json::Value::Null,
        });
        let cleared = note_from_status(&saved, &switched, "/books/b.epub");
        assert!(cleared.passage.is_none());
        let other = spike_library::margin::Visible {
            book: spike_library::margin::LiveBook {
                book: "/books/b.epub".into(),
                gen: passage.gen + 1,
            },
            page: 4,
            spine: String::new(),
            moving: false,
        };
        let unselected = commit_margin(
            &spike_library::margin::NoteField {
                body: "摘录".into(),
                ..cleared
            },
            "摘录",
            &other,
        );
        assert_eq!(unselected.body, "摘录");
        assert_eq!(unselected.notice, spike_library::margin::NOTICE_SELECT);
        let mut turning = place.clone();
        turning.moving = true;
        let mid_turn = commit_margin(
            &spike_library::margin::NoteField {
                body: "别丢".into(),
                passage: Some(passage.clone()),
                ..spike_library::margin::NoteField::default()
            },
            "别丢",
            &turning,
        );
        assert_eq!(mid_turn.body, "别丢");
        assert_eq!(mid_turn.notice, spike_library::margin::NOTICE_STALE);
        let mut left = place.clone();
        left.page += 1;
        let departed = commit_margin(
            &spike_library::margin::NoteField {
                body: "别丢".into(),
                passage: Some(passage.clone()),
                ..spike_library::margin::NoteField::default()
            },
            "别丢",
            &left,
        );
        assert_eq!(departed.body, "别丢");
        assert_eq!(departed.notice, spike_library::margin::NOTICE_STALE);
        let stale = commit_margin(
            &spike_library::margin::NoteField {
                body: "摘录".into(),
                passage: Some(passage),
                ..spike_library::margin::NoteField::default()
            },
            "摘录",
            &other,
        );
        assert_eq!(stale.body, "摘录");
        assert_eq!(stale.notice, spike_library::margin::NOTICE_STALE);
        assert!(load_margin("/books/b.epub", 4, "").unwrap().is_empty());
        assert_eq!(load_margin("/books/a.pdf", 4, "").unwrap().len(), 1);
        drop(restore);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn commit_card_updates_and_replies_through_the_support_file() {
        struct Restore(Option<String>);
        impl Drop for Restore {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
                    None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
                }
            }
        }
        let restore = Restore(std::env::var("SUPERREADER_SUPPORT_DIR").ok());
        let dir = std::env::temp_dir().join(format!("margin-card-shell-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        let notes = spike_library::margin::Notes::open(&dir.join("annotations.sqlite")).unwrap();
        let id = notes
            .insert(&spike_library::margin::NewNote {
                book: "/books/a.pdf".into(),
                page: 4,
                spine: String::new(),
                start: 0,
                end: 3,
                passage: "ink".into(),
                body: "旧批注".into(),
                author: "读者".into(),
                quads: vec![spike_library::margin::Quad {
                    x: 1.0,
                    y: 2.0,
                    w: 3.0,
                    h: 4.0,
                }],
                text_right: 8.0,
            })
            .unwrap();
        drop(notes);
        let live = spike_library::margin::LivePage {
            book: "/books/a.pdf".into(),
            page: 4,
            ..spike_library::margin::LivePage::default()
        };
        let placed = absorb_margin(
            &spike_library::margin::MarginInk::default(),
            &serde_json::json!({"book_gen": 1, "passage": serde_json::Value::Null}),
            &live,
        );
        let mut drafting = placed.clone();
        drafting.new_note.body = "未发送".into();
        let key = spike_library::margin::CardKey {
            book: "/books/a.pdf".into(),
            note_id: id,
            kind: spike_library::margin::CardKind::Edit,
        };
        let editing = spike_library::margin::begin_card(&drafting, key.clone(), "旧批注");
        let visible = spike_library::margin::Visible {
            book: spike_library::margin::LiveBook {
                book: "/books/a.pdf".into(),
                gen: 1,
            },
            page: 4,
            spine: String::new(),
            moving: false,
        };
        let blocked = commit_ink(&editing, "未发送", &visible);
        assert!(matches!(blocked.face, spike_library::margin::Face::Card(_)));
        assert_eq!(load_margin("/books/a.pdf", 4, "").unwrap().len(), 1);
        let saved = commit_card(&editing, &key, "新批注");
        assert!(matches!(saved.face, spike_library::margin::Face::New));
        assert_eq!(
            load_margin("/books/a.pdf", 4, "").unwrap()[0].body,
            "新批注"
        );
        let reply_key = spike_library::margin::CardKey {
            kind: spike_library::margin::CardKind::Reply,
            ..key
        };
        let replying = spike_library::margin::begin_card(&saved, reply_key.clone(), "");
        let replied = commit_card(&replying, &reply_key, "一条回复");
        assert!(matches!(replied.face, spike_library::margin::Face::New));
        let again = load_margin("/books/a.pdf", 4, "").unwrap();
        assert_eq!(again[0].body, "新批注");
        assert_eq!(again[0].replies, vec!["一条回复".to_string()]);
        let turned = spike_library::margin::LivePage { page: 5, ..live };
        let left = absorb_margin(&replying, &serde_json::json!({}), &turned);
        assert!(matches!(left.face, spike_library::margin::Face::New));
        assert!(left.draft(&reply_key).is_some());
        let late = commit_card(&left, &reply_key, "迟到");
        assert_eq!(
            load_margin("/books/a.pdf", 4, "").unwrap()[0].replies.len(),
            1
        );
        assert!(matches!(late.face, spike_library::margin::Face::New));
        drop(restore);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_second_draft_stays_visible_and_the_card_still_opens() {
        struct Restore(Option<String>);
        impl Drop for Restore {
            fn drop(&mut self) {
                match self.0.take() {
                    Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
                    None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
                }
            }
        }
        let restore = Restore(std::env::var("SUPERREADER_SUPPORT_DIR").ok());
        let dir = std::env::temp_dir().join(format!("margin-held-shell-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        let passage = spike_library::margin::Passage {
            book: "/books/a.pdf".into(),
            gen: 1,
            page: 4,
            spine: String::new(),
            start: 0,
            end: 3,
            text: "ink".into(),
            quads: vec![spike_library::margin::Quad {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            }],
            text_right: 8.0,
        };
        let id = spike_library::margin::Notes::open(&dir.join("annotations.sqlite"))
            .unwrap()
            .insert(&spike_library::margin::NewNote {
                book: passage.book.clone(),
                page: passage.page,
                spine: passage.spine.clone(),
                start: passage.start,
                end: passage.end,
                passage: passage.text.clone(),
                body: "旧批注".into(),
                author: "读者".into(),
                quads: passage.quads.clone(),
                text_right: passage.text_right,
            })
            .unwrap();
        let live = spike_library::margin::LivePage {
            book: passage.book.clone(),
            page: passage.page,
            ..spike_library::margin::LivePage::default()
        };
        let placed = absorb_margin(
            &spike_library::margin::MarginInk::default(),
            &serde_json::json!({"book_gen": 1, "passage": serde_json::Value::Null}),
            &live,
        );
        let mut first = placed;
        first.new_note.body = "未发送".into();
        first.new_note.passage = Some(passage.clone());
        let key = spike_library::margin::CardKey {
            book: passage.book.clone(),
            note_id: id,
            kind: spike_library::margin::CardKind::Edit,
        };
        let stashed = spike_library::margin::begin_card(&first, key.clone(), "旧批注");
        let waiting = spike_library::margin::cancel_card(&stashed, &key);
        let mut second = waiting;
        second.new_note.body = "第二条".into();
        let opened = spike_library::margin::begin_card(&second, key.clone(), "旧批注");
        assert!(
            matches!(opened.face, spike_library::margin::Face::Card(_)),
            "新建已收起"
        );
        assert_eq!(opened.new_note.body, "第二条");
        assert_eq!(opened.parked.as_ref().unwrap().body, "未发送");
        let field = spike_library::margin::cancel_card(&opened, &key);
        let told = spike_library::margin::restore_new(&field);
        assert_eq!(told.notice_line(), spike_library::margin::NOTICE_HELD);
        assert_eq!(told.new_note.body, "第二条");
        assert_eq!(told.parked.as_ref().unwrap().body, "未发送");
        let stale_page = spike_library::margin::Visible {
            book: spike_library::margin::LiveBook {
                book: passage.book.clone(),
                gen: 1,
            },
            page: passage.page + 1,
            spine: String::new(),
            moving: false,
        };
        let stale = commit_ink(&told, "第二条", &stale_page);
        assert_eq!(stale.notice_line(), spike_library::margin::NOTICE_STALE);
        assert_eq!(stale.new_note.body, "第二条");
        assert_eq!(stale.parked.as_ref().unwrap().body, "未发送");
        assert_eq!(load_margin(&passage.book, passage.page, "").unwrap().len(), 1);
        let still = spike_library::margin::begin_card(&stale, key.clone(), "种子");
        assert!(matches!(still.face, spike_library::margin::Face::Card(_)));
        let bare_face = spike_library::margin::cancel_card(&still, &key);
        let mut bare = bare_face;
        bare.new_note.passage = None;
        let here = spike_library::margin::Visible {
            page: passage.page,
            ..stale_page
        };
        let missing = commit_ink(&bare, "第二条", &here);
        assert_eq!(missing.notice_line(), spike_library::margin::NOTICE_SELECT);
        assert_eq!(missing.new_note.body, "第二条");
        assert_eq!(load_margin(&passage.book, passage.page, "").unwrap().len(), 1);
        let reachable = spike_library::margin::begin_card(&missing, key.clone(), "旧批注");
        assert!(
            matches!(reachable.face, spike_library::margin::Face::Card(_)),
            "新建已收起"
        );
        let typed = spike_library::margin::type_card(&reachable, &key, "改过");
        let other = spike_library::margin::LivePage {
            book: "other.epub".into(),
            page: 1,
            ..live
        };
        let left = absorb_margin(&typed, &serde_json::json!({}), &other);
        assert!(matches!(left.face, spike_library::margin::Face::New));
        assert_eq!(left.draft(&key).unwrap().body, "改过");
        assert_eq!(left.draft(&key).unwrap().key.book, passage.book);
        assert_eq!(left.new_note.body, "第二条");
        assert_eq!(left.parked.as_ref().unwrap().body, "未发送");
        let again = spike_library::margin::begin_card(&left, key.clone(), "不该覆盖");
        assert_eq!(again.draft(&key).unwrap().body, "改过");
        std::fs::write(dir.join("annotations.sqlite"), b"not a database").unwrap();
        let kept = commit_card(&again, &key, "改过");
        assert!(matches!(kept.face, spike_library::margin::Face::Card(_)));
        assert_eq!(kept.notice_line(), spike_library::margin::NOTICE_KEPT);
        assert_eq!(kept.new_note.body, "第二条");
        assert_eq!(kept.parked.as_ref().unwrap().body, "未发送");
        drop(restore);
        let _ = std::fs::remove_dir_all(dir);
    }
}
