//! The shell is one form. `step` is the only writer. `place` is the geometry.
//! `mount` paints that geometry and does not invent a second one.

use std::path::PathBuf;

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
}

pub fn launch() -> Form {
    Form::Full {
        sidebar: Presence::Shown,
        chat: Presence::Shown,
    }
}

pub fn step(form: Form, command: Command) -> Form {
    match (form, command) {
        (Form::Immersive, _) => Form::Immersive,
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
    pub traffic: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub gray: bool,
    pub sidebar: Option<Rect>,
    pub card: CardBox,
    pub chat: Option<Rect>,
    pub traffic: f64,
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
    AwaitingMargin {
        left: f64,
        top: f64,
        bottom: f64,
    },
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
            if margin.shown() {
                let left = if sidebar.shown() { SIDEBAR_W } else { 0.0 };
                return Placement {
                    gray: true,
                    sidebar: sidebar_rect(sidebar.shown(), window),
                    card: CardBox::AwaitingMargin {
                        left: left + INSET,
                        top: INSET,
                        bottom: INSET,
                    },
                    chat: None,
                    traffic: input.traffic,
                    face: CardFace::Page,
                    rows: rows(library),
                    groups: groups(),
                };
            }
            (
                if sidebar.shown() { SIDEBAR_W } else { 0.0 },
                0.0,
                INSET,
                RADIUS,
                true,
                true,
            )
        }
        Form::Immersive => (0.0, 0.0, 0.0, 0.0, false, false),
    };
    let rect = Rect {
        x: sidebar_w + inset,
        y: inset,
        w: (window.width - sidebar_w - chat_w - inset * 2.0).max(1.0),
        h: (window.height - inset * 2.0).max(1.0),
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
                y: 0.0,
                w: chat_w,
                h: window.height,
            })
        } else {
            None
        },
        traffic: input.traffic,
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

pub fn mount(
    form: Signal<Form>,
    selected: Signal<SelectId>,
    window: Signal<Window>,
    traffic: Signal<f64>,
    library: Signal<LibraryIn>,
    menu: Signal<bool>,
    on_peer: impl Fn(Peer) + Clone + 'static,
    on_open: impl Fn(String) + Clone + 'static,
) -> impl IntoElement {
    let form_b = form;
    let selected_b = selected;
    let window_b = window;
    let traffic_b = traffic;
    let library_b = library;
    let menu_b = menu;
    let on_peer_b = on_peer;
    let on_open_b = on_open;
    dynamic(move || {
        let current = form_b.get();
        let library = library_b.get();
        let placed = place(
            &PlaceIn {
                form: current,
                window: window_b.get(),
                traffic: traffic_b.get(),
            },
            &library,
        );
        write_layout(&placed, &library);
        paint(
            placed,
            form_b,
            selected_b,
            menu_b,
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
        if notes.iter().any(|note| note.label == label) {
            return;
        }
        notes.push(FontNote {
            label: label.to_string(),
            size,
            weight: weight.0,
        });
    }
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
    let card = match placed.card {
        CardBox::Placed { rect, .. } => {
            serde_json::json!([rect.x, rect.y, rect.w, rect.h])
        }
        CardBox::AwaitingMargin { .. } => serde_json::json!(null),
    };
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
        let row_w = (side.w as f32 - crate::tokens::SIDE_PAD_X * 2.0).max(0.0);
        for (index, row) in placed.rows.iter().enumerate() {
            let y = 80.0 + index as f32 * crate::tokens::ROW_H;
            let frame = [crate::tokens::SIDE_PAD_X, y, row_w, crate::tokens::ROW_H];
            nodes.push(layout_node(
                &format!("sidebar.row.{index}"),
                "sidebar-row",
                frame,
                [row_w, crate::tokens::ROW_H],
                side_frame,
                false,
                &row.label,
                true,
            ));
            let slot = (row_w - crate::tokens::ROW_ICON_SLOT - crate::tokens::ROW_GAP * 2.0 - 24.0)
                .max(0.0);
            nodes.push(layout_node(
                &format!("sidebar.row.{index}.label"),
                "text",
                [
                    crate::tokens::SIDE_PAD_X
                        + crate::tokens::ROW_ICON_SLOT
                        + crate::tokens::ROW_GAP,
                    y,
                    slot,
                    crate::tokens::ROW_H,
                ],
                [
                    text_width(&row.label, crate::tokens::BODY_SIZE),
                    crate::tokens::BODY_SIZE,
                ],
                frame,
                true,
                &row.label,
                true,
            ));
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
        nodes.push(layout_node(
            "chat.name",
            "text",
            [chat.x as f32 + 16.0, chat.y as f32 + 12.0, 120.0, 20.0],
            [
                text_width("Slowpage", crate::tokens::NAME_SIZE),
                crate::tokens::NAME_SIZE,
            ],
            [chat.x as f32, chat.y as f32, chat.w as f32, chat.h as f32],
            true,
            "Slowpage",
            false,
        ));
    }
    let grid = spike_library::shelf_grid(content_w);
    let row_w = spike_library::row_width(grid);
    nodes.push(layout_node(
        "shelf.row",
        "container",
        [origin_x, content[1], row_w, grid.cover_h + grid.caption_h],
        [row_w, grid.cover_h],
        content,
        false,
        "",
        true,
    ));
    for index in 0..grid.columns.min(library.covers.len() as u32) {
        let x = origin_x + index as f32 * (grid.cover_w + grid.column_gap);
        let frame = [x, content[1], grid.cover_w, grid.cover_h];
        let title = library
            .covers
            .get(index as usize)
            .map(|card| card.title.as_str())
            .unwrap_or("");
        nodes.push(layout_node(
            &format!("cover.{index}"),
            "cover",
            frame,
            [grid.cover_w, grid.cover_h],
            content,
            false,
            title,
            true,
        ));
        nodes.push(layout_node(
            &format!("cover.{index}.title"),
            "text",
            [x, content[1] + grid.cover_h, grid.cover_w, grid.caption_h],
            [
                text_width(title, crate::tokens::SHELF_TITLE_SIZE),
                crate::tokens::SHELF_TITLE_SIZE,
            ],
            [x, content[1] + grid.cover_h, grid.cover_w, grid.caption_h],
            true,
            title,
            true,
        ));
    }
    let continued =
        crate::tokens::continue_layout(content_w, library.continue_reading.len() as u32);
    for index in 0..continued.cards.min(library.continue_reading.len() as u32) {
        let x = origin_x + index as f32 * (continued.card_w + continued.gap);
        let frame = [x, content[1], continued.card_w, continued.cover_h + 24.0];
        let title = library
            .continue_reading
            .get(index as usize)
            .map(|card| card.title.as_str())
            .unwrap_or("");
        nodes.push(layout_node(
            &format!("continue.{index}"),
            "button",
            frame,
            [continued.card_w, continued.cover_h],
            content,
            false,
            title,
            true,
        ));
    }
    nodes
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
    let body = serde_json::json!({
        "window": [window_w, window_h],
        "nodes": layout_nodes(placed, library),
    });
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
        root = root.child(sidebar_toggle_button(placed.traffic, form));
    }
    if placed.chat.is_none() && matches!(form.get(), Form::Full { .. }) {
        root = root.child(panel_toggle_button(form, true));
    }
    root.into_element()
}

fn sidebar_pane(
    rect: Option<Rect>,
    placed: &Placement,
    form: Signal<Form>,
    selected: Signal<SelectId>,
    menu: Signal<bool>,
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
    column = column.child(sidebar_top(placed.traffic, form));
    column = column.child(
        view()
            .px(px(crate::tokens::SIDE_PAD_X))
            .child(search_field()),
    );
    let mut groups = scroll()
        .flex_1()
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
        .left(px(16.0))
        .bottom(px(52.0))
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

fn sidebar_top(traffic: f64, form: Signal<Form>) -> AnyElement {
    let leading = if traffic > 8.0 { traffic } else { 78.0 };
    view()
        .h(px(crate::tokens::TOP_ROW_H))
        .flex()
        .flex_row()
        .items_center()
        .child(view().w(px(leading as f32)).h(px(crate::tokens::TOP_ROW_H)))
        .child(sidebar_toggle_button(0.0, form))
        .into_element()
}

fn sidebar_toggle_button(traffic: f64, form: Signal<Form>) -> AnyElement {
    let mut button = view()
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
    if traffic > 0.0 {
        button = button
            .absolute()
            .left(px(traffic as f32))
            .top(px(crate::tokens::TOGGLE_TOP));
    }
    if let Some(path) = crate::icons::glyph_ink("sidebar-left", "6E6E73") {
        button = button.child(
            image(format!("file://{}", path.display())).size(px(crate::tokens::SIDEBAR_ICON)),
        );
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

fn panel_toggle_button(form: Signal<Form>, floating: bool) -> AnyElement {
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
            .top(px(crate::tokens::TOGGLE_TOP));
    }
    if let Some(path) = crate::icons::glyph_ink("sidebar-right", "6E6E73") {
        button = button
            .child(image(format!("file://{}", path.display())).size(px(crate::tokens::PANEL_ICON)));
    }
    button.into_element()
}

fn search_field() -> AnyElement {
    view()
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
        .child(view().flex_1().min_w(px(0.0)).child(fit_text(
            "搜索书名、作者、笔记".to_string(),
            crate::tokens::SEARCH_SIZE,
            crate::tokens::BODY_WEIGHT,
            quiet(),
            crate::tokens::BODY_TRACK_EM,
        )))
        .into_element()
}

fn group_label(label: &str) -> AnyElement {
    view()
        .w_full()
        .min_w(px(0.0))
        .pt(px(crate::tokens::LABEL_MT))
        .pb(px(crate::tokens::LABEL_MB))
        .px(px(crate::tokens::LABEL_PX))
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
        Some(path) => image(format!("file://{}", path.display()))
            .size(px(size))
            .into_element(),
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
                .child(panel_toggle_button(form, false)),
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
        button = button.child(image(format!("file://{}", path.display())).size(px(size)));
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
        .flex()
        .flex_row()
        .items_center()
        .gap(px(crate::tokens::COMPOSER_ROW_GAP))
        .child(icon_button("paperclip", "附件", crate::tokens::ATTACH_ICON));
    if !book.is_empty() {
        tools = tools.child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .h(px(crate::tokens::SCOPE_H))
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
                ))),
        );
    }
    if book.is_empty() {
        tools = tools.child(view().flex_1());
    }
    tools = tools.child(send_button());
    view()
        .absolute()
        .left(px(crate::tokens::COMPOSER_INSET))
        .right(px(crate::tokens::COMPOSER_INSET))
        .bottom(px(crate::tokens::COMPOSER_INSET))
        .rounded(px(crate::tokens::COMPOSER_RADIUS))
        .pt(px(crate::tokens::COMPOSER_PT))
        .pr(px(crate::tokens::COMPOSER_PR))
        .pb(px(crate::tokens::COMPOSER_PB))
        .pl(px(crate::tokens::COMPOSER_PL))
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
        button = button
            .child(image(format!("file://{}", path.display())).size(px(crate::tokens::SEND_ICON)));
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
        CardFace::Grid => column.child(
            scroll()
                .flex_1()
                .w_full()
                .flex()
                .flex_col()
                .scrollable(true)
                .child(book_grid(library, content_w, on_open)),
        ),
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
        control = control
            .child(image(format!("file://{}", path.display())).size(px(crate::tokens::SORT_ICON)));
    }
    control = control.child(view().flex_1().min_w(px(0.0)).child(fit_text(
        "最近阅读".to_string(),
        crate::tokens::TOOL_SIZE,
        crate::tokens::BODY_WEIGHT,
        icon_ink(),
        crate::tokens::BODY_TRACK_EM,
    )));
    if let Some(path) = crate::icons::glyph_ink("chevron-down", "6E6E73") {
        control = control.child(
            image(format!("file://{}", path.display())).size(px(crate::tokens::CHEVRON_ICON)),
        );
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
        button = button
            .child(image(format!("file://{}", path.display())).size(px(crate::tokens::VIEW_ICON)));
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
        .child(view().flex_1().min_w(px(0.0)).child(fit_text(
            title.to_string(),
            crate::tokens::H1_SIZE,
            crate::tokens::H1_WEIGHT,
            ink(),
            crate::tokens::H1_TRACK_EM,
        )))
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
    let sampled = card.plate_rgb.or_else(|| sample_cover(card));
    let color = sampled.unwrap_or_else(|| fallback_color(&card.title));
    let photo = card.cover.is_some() && card.plate_rgb.is_none() && !card.pdf;
    let (title_ink, author_ink) = plate_inks(color);
    let mut art = view()
        .w(px(width))
        .h(px(height))
        .rounded(px(crate::tokens::SHELF_RADIUS))
        .overflow_hidden()
        .shadow(crate::tokens::cover_shadows())
        .bg(rgb(color).to_hsla());
    if card.cover.is_none() {
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
    if let Some(path) = &card.cover {
        art = art.child(
            image(format!("file://{}", path.display()))
                .w(px(width))
                .h(px(height)),
        );
    }
    if !photo {
        let title_size = if height < 100.0 {
            crate::tokens::TAG_SM_SIZE
        } else {
            crate::tokens::SECTION_SIZE
        };
        let author_size = if height < 100.0 {
            crate::tokens::TAG_SM_SIZE
        } else {
            crate::tokens::TAG_SIZE
        };
        art = art.child(
            view()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px(px(crate::tokens::COVER_LABEL_PX))
                .gap(px(crate::tokens::SHELF_META_GAP))
                .child(fit_text(
                    card.title.clone(),
                    title_size,
                    crate::tokens::H1_WEIGHT,
                    title_ink,
                    0.0,
                ))
                .child(fit_text(
                    card.author.clone(),
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
                traffic: 92.0,
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
                    height: crate::tokens::MIN_WINDOW_H,
                },
                traffic: 78.0,
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
    }

    #[test]
    fn hiding_a_column_keeps_the_six_point_inset() {
        let hidden_side = step(launch(), Command::ToggleSidebar);
        let placed = place(
            &PlaceIn {
                form: hidden_side,
                window: window(),
                traffic: 92.0,
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
                traffic: 92.0,
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
                traffic: 0.0,
            },
            &empty_library(),
        );
        assert!(placed.chat.is_none());
        assert!(matches!(placed.card, CardBox::AwaitingMargin { .. }));
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
                traffic: 0.0,
            },
            &library,
        );
        assert_eq!(placed.face, CardFace::Grid);
    }
}
