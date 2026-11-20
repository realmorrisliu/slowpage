//! Annotation cards. Geometry is the 2026-11-20 22:42 set. Storage is the
//! local SQLite the route already assigned to SuperReader. `on_return` is the
//! only save. This module does not call an agent and does not ship sample notes.

use std::path::Path;

use rusqlite::{params, Connection};

pub const MARGIN_W: f64 = 240.0;
pub const CARD_X: f64 = 16.0;
pub const CARD_W: f64 = 208.0;
pub const GAP: f64 = 8.0;
pub const RADIUS: f64 = 12.0;
pub const BAND_TOP: f64 = 48.0;
pub const COMPOSER_H: f64 = 88.0;
pub const COMPOSER_INSET: f64 = 18.0;
pub const LINE_H: f64 = 18.0;
pub const PAD_Y: f64 = 8.0;
pub const INNER_W: f64 = CARD_W - 24.0;
pub const CONNECTOR_FADE: f64 = 0.28;

const CHAR_W: f64 = 13.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBox {
    pub top: f64,
    pub height: f64,
    pub highlight_right: f64,
    pub text_right: f64,
}

/// Right end of the connector on one line. A later line's quad is not an input.
pub fn connector_x(highlight_right: f64, text_right: f64) -> f64 {
    highlight_right.max(text_right)
}

/// First line of a highlight only. A wrapped fragment on the next line is ignored.
pub fn first_line(quads: &[Quad], text_right: f64) -> Option<LineBox> {
    let first = quads.iter().min_by(|a, b| a.y.total_cmp(&b.y))?;
    let mut right = f64::MIN;
    let mut height = first.h;
    for quad in quads {
        if (quad.y - first.y).abs() < 0.5 {
            right = right.max(quad.x + quad.w);
            height = height.max(quad.h);
        }
    }
    if !right.is_finite() {
        return None;
    }
    Some(LineBox {
        top: first.y,
        height,
        highlight_right: right,
        text_right: connector_x(right, text_right),
    })
}

pub fn band_bottom(window_h: f64) -> f64 {
    window_h - COMPOSER_INSET - COMPOSER_H
}

pub fn collapsed_height() -> f64 {
    PAD_Y + LINE_H + LINE_H + PAD_Y
}

fn wrap_lines(text: &str) -> usize {
    let per = (INNER_W / CHAR_W).floor().max(1.0) as usize;
    let mut lines = 0usize;
    if text.is_empty() {
        return 1;
    }
    for para in text.split('\n') {
        let count = para.chars().count().max(1);
        lines += count.div_ceil(per);
    }
    lines.max(1)
}

pub fn shown_text(note: &StoredNote) -> &str {
    if note.body.trim().is_empty() {
        &note.passage
    } else {
        &note.body
    }
}

pub fn open_height(body: &str, replies: &[String]) -> f64 {
    let mut height = PAD_Y + LINE_H + (wrap_lines(body) as f64 + 1.0) * LINE_H + PAD_Y;
    for reply in replies {
        height += GAP + PAD_Y + (wrap_lines(reply) as f64 + 1.0) * LINE_H;
    }
    height
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardIn {
    pub id: i64,
    pub line: LineBox,
    pub collapsed_h: f64,
    pub hover_h: f64,
    pub selected_h: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Focus {
    pub hover: Option<i64>,
    pub selected: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub fade: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub id: i64,
    pub x: f64,
    pub y: f64,
    pub h: f64,
    pub open: bool,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub cards: Vec<Placed>,
    pub scroll: f64,
    pub scroll_min: f64,
    pub scroll_max: f64,
    pub band_top: f64,
    pub band_bottom: f64,
}

struct Draft {
    id: i64,
    line: LineBox,
    h: f64,
    y: f64,
    open: bool,
    selected: bool,
    hover: bool,
}

/// `margin_x` is the column's left edge in the window. `scroll` is zero when
/// cards sit on their lines. Any other scroll hides connectors.
pub fn place(cards: &[CardIn], focus: Focus, window_h: f64, margin_x: f64, scroll: f64) -> Scene {
    let band_top = BAND_TOP;
    let bottom = band_bottom(window_h);
    let mut drafts: Vec<Draft> = cards
        .iter()
        .map(|card| {
            let selected = focus.selected == Some(card.id);
            let hover = focus.hover == Some(card.id);
            let h = if selected {
                card.selected_h
            } else if hover {
                card.hover_h
            } else {
                card.collapsed_h
            };
            Draft {
                id: card.id,
                line: card.line,
                h,
                y: parked(card.line.top, h, band_top, bottom),
                open: selected || hover,
                selected,
                hover,
            }
        })
        .collect();
    drafts.sort_by(|a, b| a.line.top.total_cmp(&b.line.top).then(a.id.cmp(&b.id)));
    push_down(&mut drafts);
    if let Some(index) = drafts.iter().position(|card| card.selected) {
        let top = drafts[index].line.top;
        let h = drafts[index].h;
        drafts[index].y = if top >= band_top && top < bottom {
            top
        } else {
            parked(top, h, band_top, bottom)
        };
        push_from(&mut drafts, index);
    }
    pull_up(&mut drafts);
    let (scroll_min, scroll_max) = scroll_limits(&drafts, band_top, bottom);
    let scroll = if drafts.is_empty() {
        0.0
    } else {
        scroll.clamp(scroll_min, scroll_max)
    };
    let at_rest = scroll.abs() < 0.5;
    let card_x = margin_x + CARD_X;
    let placed = drafts
        .iter()
        .map(|card| Placed {
            id: card.id,
            x: card_x,
            y: card.y - scroll,
            h: card.h,
            open: card.open,
            segments: if at_rest {
                segments(card, card_x)
            } else {
                Vec::new()
            },
        })
        .collect();
    Scene {
        cards: placed,
        scroll,
        scroll_min,
        scroll_max,
        band_top,
        band_bottom: bottom,
    }
}

fn parked(line_top: f64, h: f64, band_top: f64, band_bottom: f64) -> f64 {
    if line_top < band_top {
        band_top
    } else if line_top >= band_bottom {
        (band_bottom - GAP - h).max(band_top)
    } else {
        line_top
    }
}

fn push_down(cards: &mut [Draft]) {
    push_from(cards, 0);
}

fn push_from(cards: &mut [Draft], start: usize) {
    for index in start.saturating_add(1)..cards.len() {
        let min = cards[index - 1].y + cards[index - 1].h + GAP;
        if cards[index].y < min {
            cards[index].y = min;
        }
    }
}

fn pull_up(cards: &mut [Draft]) {
    let Some(selected) = cards.iter().position(|card| card.selected) else {
        return;
    };
    for index in (0..selected).rev() {
        let limit = cards[index + 1].y - GAP - cards[index].h;
        if cards[index].y > limit {
            cards[index].y = limit;
        }
    }
}

fn scroll_limits(cards: &[Draft], band_top: f64, band_bottom: f64) -> (f64, f64) {
    let Some(top) = cards.iter().map(|card| card.y).reduce(f64::min) else {
        return (0.0, 0.0);
    };
    let bot = cards
        .iter()
        .map(|card| card.y + card.h)
        .reduce(f64::max)
        .unwrap_or(top);
    let scroll_min = (top - band_top).min(0.0);
    let scroll_max = (bot - (band_bottom - GAP)).max(0.0);
    (scroll_min, scroll_max)
}

fn segments(card: &Draft, card_x: f64) -> Vec<Segment> {
    if !card.selected && !card.hover {
        return Vec::new();
    }
    let fade = card.hover && !card.selected;
    let y_line = card.line.top + card.line.height / 2.0;
    let x0 = card.line.text_right;
    if x0 + 0.5 >= card_x {
        return Vec::new();
    }
    let parked = (card.y - card.line.top).abs() > 1.0;
    if !parked {
        return horizontal(x0, card_x, y_line, fade).into_iter().collect();
    }
    let knee = (card_x - 10.0).max(x0);
    let y_card = card.y + LINE_H.min(card.h / 2.0);
    let mut out = Vec::new();
    if let Some(seg) = horizontal(x0, knee, y_line, fade) {
        out.push(seg);
    }
    let top = y_line.min(y_card);
    let height = (y_card - y_line).abs().max(1.0);
    out.push(Segment {
        x: knee,
        y: top,
        w: 1.0,
        h: height,
        fade,
    });
    if let Some(seg) = horizontal(knee, card_x, y_card, fade) {
        out.push(seg);
    }
    out
}

fn horizontal(from: f64, to: f64, y: f64, fade: bool) -> Option<Segment> {
    if to <= from + 0.5 {
        return None;
    }
    Some(Segment {
        x: from,
        y: y - 0.5,
        w: to - from,
        h: 1.0,
        fade,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct LivePage {
    pub book: String,
    pub spine: String,
    pub page: u32,
    pub page_w: f64,
    pub page_h: f64,
    pub moving: bool,
}

impl Default for LivePage {
    fn default() -> Self {
        Self {
            book: String::new(),
            spine: String::new(),
            page: 0,
            page_w: 0.0,
            page_h: 0.0,
            moving: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hold {
    pub book: String,
    pub spine: String,
    pub page: u32,
    pub moving: bool,
    pub scroll: f64,
    pub hover: Option<i64>,
    pub selected: Option<i64>,
    pub scroll_min: f64,
    pub scroll_max: f64,
}

impl Hold {
    pub fn new() -> Self {
        Self {
            book: String::new(),
            spine: String::new(),
            page: 0,
            moving: false,
            scroll: 0.0,
            hover: None,
            selected: None,
            scroll_min: 0.0,
            scroll_max: 0.0,
        }
    }
}

impl Default for Hold {
    fn default() -> Self {
        Self::new()
    }
}

/// While a page turn or an immersive trip is moving, keep the departing page.
/// Landing takes the arrived page and returns the scroll to the lines.
pub fn reconcile(hold: &Hold, live: &LivePage) -> Hold {
    if hold.book != live.book || hold.spine != live.spine {
        return Hold {
            book: live.book.clone(),
            spine: live.spine.clone(),
            page: live.page,
            moving: live.moving,
            ..Hold::new()
        };
    }
    if live.moving {
        return Hold {
            moving: true,
            page: if hold.page == 0 { live.page } else { hold.page },
            ..hold.clone()
        };
    }
    if hold.moving || hold.page != live.page {
        return Hold {
            book: live.book.clone(),
            spine: live.spine.clone(),
            page: live.page,
            moving: false,
            ..Hold::new()
        };
    }
    Hold {
        moving: false,
        ..hold.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub origin_x: f64,
    pub origin_y: f64,
    pub scale: f64,
}

/// Map a page-point quad through the same fit the page bitmap uses.
pub fn map_quad(quad: Quad, fit: Fit) -> Quad {
    Quad {
        x: fit.origin_x + quad.x * fit.scale,
        y: fit.origin_y + quad.y * fit.scale,
        w: quad.w * fit.scale,
        h: quad.h * fit.scale,
    }
}

pub fn map_x(x: f64, fit: Fit) -> f64 {
    fit.origin_x + x * fit.scale
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewNote {
    pub book: String,
    pub page: u32,
    pub spine: String,
    pub start: u32,
    pub end: u32,
    pub passage: String,
    pub body: String,
    pub author: String,
    pub quads: Vec<Quad>,
    pub text_right: f64,
}

/// The reader who typed the note. There is no account on this save.
pub const LOCAL_AUTHOR: &str = "读者";

pub const NOTICE_BLANK: &str = "先写一句笔记";
pub const NOTICE_SELECT: &str = "先选中原文";
pub const NOTICE_STALE: &str = "选区已经换了";

/// One PDF glyph in page points. The index is a Unicode scalar offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBox {
    pub index: u32,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// One measured EPUB line box, in the page's own points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasuredRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The selection captured with the book that was open. `gen` is that open's
/// session token. It is not stored.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    pub book: String,
    pub gen: u64,
    pub page: u32,
    pub spine: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub quads: Vec<Quad>,
    pub text_right: f64,
}

/// Unchecked geometry. [`passage_from`] is what makes it a [`Passage`].
#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    pub book: String,
    pub gen: u64,
    pub page: u32,
    pub spine: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub quads: Vec<Quad>,
    pub text_right: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub book: String,
    pub page: u32,
    pub start: u32,
    pub end: u32,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveBook {
    pub book: String,
    pub gen: u64,
}

/// The page the reader is looking at. A note saves only while its passage is this page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visible {
    pub book: LiveBook,
    pub page: u32,
    pub spine: String,
    pub moving: bool,
}

/// Line boxes measured from the current layout. Stored viewport pixels are not these.
#[derive(Clone, Debug, PartialEq)]
pub struct FreshMeasure {
    pub id: i64,
    pub page: u32,
    pub quads: Vec<Quad>,
    pub text_right: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NoteField {
    pub body: String,
    pub composing: bool,
    pub passage: Option<Passage>,
    pub fingerprint: Option<Fingerprint>,
    pub notice: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnKind {
    Key,
    Blur,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsaved {
    Composing,
    Blank,
    Unselected,
    Stale,
    Duplicate,
}

impl Unsaved {
    pub fn notice(self) -> Option<&'static str> {
        match self {
            Unsaved::Composing | Unsaved::Duplicate => None,
            Unsaved::Blank => Some(NOTICE_BLANK),
            Unsaved::Unselected => Some(NOTICE_SELECT),
            Unsaved::Stale => Some(NOTICE_STALE),
        }
    }
}

pub enum Submit {
    Saved { id: i64, fingerprint: Fingerprint },
    Kept { hold: Unsaved },
    Failed { notice: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoredNote {
    pub id: i64,
    pub page: u32,
    pub spine: String,
    pub start: u32,
    pub end: u32,
    pub passage: String,
    pub body: String,
    pub author: String,
    pub quads: Vec<Quad>,
    pub text_right: f64,
    pub replies: Vec<String>,
}

pub struct Notes {
    conn: Connection,
}

impl Notes {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let conn = Connection::open(path).map_err(|err| err.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY,
                book TEXT NOT NULL,
                page INTEGER NOT NULL,
                spine TEXT NOT NULL,
                start_at INTEGER NOT NULL,
                end_at INTEGER NOT NULL,
                passage TEXT NOT NULL,
                body TEXT NOT NULL,
                author TEXT NOT NULL,
                quads TEXT NOT NULL,
                text_right REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS replies (
                id INTEGER PRIMARY KEY,
                note_id INTEGER NOT NULL,
                body TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS notes_book_page ON notes(book, page);",
        )
        .map_err(|err| err.to_string())?;
        Ok(Self { conn })
    }

    pub fn insert(&self, note: &NewNote) -> Result<i64, String> {
        if note.book.is_empty() || note.page == 0 {
            return Err("note needs a book and a page".into());
        }
        if note.passage.trim().is_empty() && note.body.trim().is_empty() {
            return Err("note needs a passage or a body".into());
        }
        if note.author.trim().is_empty() {
            return Err("note needs an author".into());
        }
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|err| err.to_string())?;
        tx.execute(
            "INSERT INTO notes (book, page, spine, start_at, end_at, passage, body, author, quads, text_right)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                note.book,
                note.page,
                note.spine,
                note.start,
                note.end,
                note.passage,
                note.body,
                note.author,
                encode_quads(&note.quads),
                note.text_right,
            ],
        )
        .map_err(|err| err.to_string())?;
        let id = tx.last_insert_rowid();
        tx.commit().map_err(|err| err.to_string())?;
        Ok(id)
    }

    pub fn reply(&self, note_id: i64, body: &str) -> Result<i64, String> {
        if body.trim().is_empty() {
            return Err("reply is empty".into());
        }
        let found: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM notes WHERE id = ?1",
                params![note_id],
                |row| row.get(0),
            )
            .map_err(|err| err.to_string())?;
        if found == 0 {
            return Err("no such note".into());
        }
        self.conn
            .execute(
                "INSERT INTO replies (note_id, body) VALUES (?1, ?2)",
                params![note_id, body],
            )
            .map_err(|err| err.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_spine(&self, book: &str, spine: &str) -> Result<Vec<StoredNote>, String> {
        if book.is_empty() || spine.is_empty() {
            return Ok(Vec::new());
        }
        self.collect(
            "SELECT id, page, spine, start_at, end_at, passage, body, author, quads, text_right
             FROM notes WHERE book = ?1 AND spine = ?2 ORDER BY id",
            (book, spine),
        )
    }

    pub fn list(&self, book: &str, page: u32, spine: &str) -> Result<Vec<StoredNote>, String> {
        self.collect(
            "SELECT id, page, spine, start_at, end_at, passage, body, author, quads, text_right
             FROM notes WHERE book = ?1 AND page = ?2 AND spine = ?3 ORDER BY id",
            (book, page, spine),
        )
    }

    fn collect<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<StoredNote>, String> {
        let mut stmt = self.conn.prepare(sql).map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params, |row| {
                Ok(StoredNote {
                    id: row.get(0)?,
                    page: row.get(1)?,
                    spine: row.get(2)?,
                    start: row.get(3)?,
                    end: row.get(4)?,
                    passage: row.get(5)?,
                    body: row.get(6)?,
                    author: row.get(7)?,
                    quads: decode_quads(&row.get::<_, String>(8)?),
                    text_right: row.get(9)?,
                    replies: Vec::new(),
                })
            })
            .map_err(|err| err.to_string())?;
        let mut notes = Vec::new();
        for row in rows {
            let mut note = row.map_err(|err| err.to_string())?;
            note.replies = self.replies(note.id)?;
            notes.push(note);
        }
        Ok(notes)
    }

    fn replies(&self, note_id: i64) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT body FROM replies WHERE note_id = ?1 ORDER BY id")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![note_id], |row| row.get(0))
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }
}

const LINE_SLACK: f64 = 5.0;

/// Right edge of the first selected line. Glyphs on that line outside the
/// selection count. The next line does not.
pub fn pdf_line_end(glyphs: &[GlyphBox], start: u32, end: u32) -> Option<f64> {
    let first = glyphs
        .iter()
        .find(|glyph| glyph.index >= start && glyph.index < end)?;
    if !first.y.is_finite() {
        return None;
    }
    let mut right = f64::NEG_INFINITY;
    for glyph in glyphs {
        if (glyph.y - first.y).abs() > LINE_SLACK {
            continue;
        }
        let edge = glyph.x + glyph.w;
        if edge.is_finite() {
            right = right.max(edge);
        }
    }
    if right.is_finite() && right > 0.0 {
        Some(right)
    } else {
        None
    }
}

/// Keep rects that were actually measured. A zero box is not a line.
pub fn measured_quads(rects: &[MeasuredRect]) -> Vec<Quad> {
    rects
        .iter()
        .filter(|rect| {
            rect.x.is_finite()
                && rect.y.is_finite()
                && rect.w.is_finite()
                && rect.h.is_finite()
                && rect.w > 0.0
                && rect.h > 0.0
        })
        .map(|rect| Quad {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
        })
        .collect()
}

/// The placed EPUB view's point size. Zero stays unmapped so a card is not drawn.
pub fn epub_page_size(page_w: f64, page_h: f64) -> (f64, f64) {
    if page_w.is_finite() && page_h.is_finite() && page_w >= 1.0 && page_h >= 1.0 {
        (page_w, page_h)
    } else {
        (0.0, 0.0)
    }
}

pub fn passage_from(selection: Selection) -> Option<Passage> {
    if selection.gen == 0 || selection.book.is_empty() || selection.page == 0 {
        return None;
    }
    if selection.end <= selection.start {
        return None;
    }
    if selection.text.chars().count() != (selection.end - selection.start) as usize {
        return None;
    }
    if selection.text.trim().is_empty() {
        return None;
    }
    if !selection.text_right.is_finite() || selection.text_right <= 0.0 {
        return None;
    }
    if selection.quads.is_empty() || selection.quads.iter().any(|quad| !quad_live(quad)) {
        return None;
    }
    Some(Passage {
        book: selection.book,
        gen: selection.gen,
        page: selection.page,
        spine: selection.spine,
        start: selection.start,
        end: selection.end,
        text: selection.text,
        quads: selection.quads,
        text_right: selection.text_right,
    })
}

fn quad_live(quad: &Quad) -> bool {
    quad.x.is_finite()
        && quad.y.is_finite()
        && quad.w.is_finite()
        && quad.h.is_finite()
        && quad.w > 0.0
        && quad.h > 0.0
}

pub fn take_passage(published: Option<Passage>, live: &LiveBook) -> Option<Passage> {
    let passage = published?;
    if passage.book.is_empty() || passage.book != live.book || passage.gen != live.gen {
        None
    } else {
        Some(passage)
    }
}

/// A keystroke replaces the draft. The last save no longer applies to it.
pub fn note_typed(field: &NoteField, text: &str) -> NoteField {
    if field.body == text {
        return field.clone();
    }
    NoteField {
        body: text.to_string(),
        fingerprint: None,
        notice: String::new(),
        ..field.clone()
    }
}

pub fn prepare_save(
    draft: &str,
    passage: Option<&Passage>,
    visible: &Visible,
    last: Option<&Fingerprint>,
    composing: bool,
) -> Result<NewNote, Unsaved> {
    if composing {
        return Err(Unsaved::Composing);
    }
    let Some(passage) = passage else {
        return Err(Unsaved::Unselected);
    };
    if visible.moving
        || passage.book != visible.book.book
        || passage.gen != visible.book.gen
        || passage.page != visible.page
        || passage.spine != visible.spine
    {
        return Err(Unsaved::Stale);
    }
    let body = draft.trim();
    if body.is_empty() {
        return Err(Unsaved::Blank);
    }
    let fingerprint = Fingerprint {
        book: passage.book.clone(),
        page: passage.page,
        start: passage.start,
        end: passage.end,
        body: body.to_string(),
    };
    // A second Return with no new keystrokes matches this and does not write.
    // Typing the same words clears it in `note_typed` and saves again.
    if last.is_some_and(|saved| saved == &fingerprint) {
        return Err(Unsaved::Duplicate);
    }
    Ok(NewNote {
        book: passage.book.clone(),
        page: passage.page,
        spine: passage.spine.clone(),
        start: passage.start,
        end: passage.end,
        passage: passage.text.clone(),
        body: body.to_string(),
        author: LOCAL_AUTHOR.into(),
        quads: passage.quads.clone(),
        text_right: passage.text_right,
    })
}

pub fn submit_note(
    path: &Path,
    draft: &str,
    passage: Option<&Passage>,
    visible: &Visible,
    last: Option<&Fingerprint>,
    composing: bool,
) -> Submit {
    let note = match prepare_save(draft, passage, visible, last, composing) {
        Ok(note) => note,
        Err(hold) => return Submit::Kept { hold },
    };
    match Notes::open(path).and_then(|notes| notes.insert(&note)) {
        Ok(id) => Submit::Saved {
            id,
            fingerprint: Fingerprint {
                book: note.book,
                page: note.page,
                start: note.start,
                end: note.end,
                body: note.body,
            },
        },
        Err(notice) => Submit::Failed { notice },
    }
}

pub fn on_return(
    kind: ReturnKind,
    field: &NoteField,
    submitted: &str,
    visible: &Visible,
    path: &Path,
) -> NoteField {
    if kind == ReturnKind::Blur {
        return field.clone();
    }
    let mut next = field.clone();
    match submit_note(
        path,
        submitted,
        field.passage.as_ref(),
        visible,
        field.fingerprint.as_ref(),
        field.composing,
    ) {
        Submit::Saved { fingerprint, .. } => {
            next.body.clear();
            next.notice.clear();
            next.fingerprint = Some(fingerprint);
        }
        Submit::Kept {
            hold: Unsaved::Duplicate | Unsaved::Composing,
        } => {}
        Submit::Kept { hold } => {
            next.body = submitted.to_string();
            if let Some(text) = hold.notice() {
                next.notice = text.to_string();
            }
        }
        Submit::Failed { notice } => {
            next.body = submitted.to_string();
            next.notice = notice;
        }
    }
    next
}

/// Cards for one EPUB column. A stored viewport box is not a line in the current layout.
pub fn epub_shown(notes: &[StoredNote], measures: &[FreshMeasure], page: u32) -> Vec<StoredNote> {
    if page == 0 {
        return Vec::new();
    }
    notes
        .iter()
        .filter_map(|note| {
            let fresh = measures
                .iter()
                .find(|measure| measure.id == note.id && measure.page == page)?;
            let mut shown = note.clone();
            shown.page = page;
            shown.quads = fresh.quads.clone();
            shown.text_right = fresh.text_right;
            Some(shown)
        })
        .collect()
}

pub fn cards_from(notes: &[StoredNote], fit: Fit) -> Vec<CardIn> {
    if fit.scale <= 0.0 {
        return Vec::new();
    }
    notes
        .iter()
        .filter_map(|note| {
            let quads: Vec<Quad> = note
                .quads
                .iter()
                .copied()
                .map(|quad| map_quad(quad, fit))
                .collect();
            let line = first_line(&quads, map_x(note.text_right, fit))?;
            let text = shown_text(note);
            Some(CardIn {
                id: note.id,
                line,
                collapsed_h: collapsed_height(),
                hover_h: open_height(text, &[]),
                selected_h: open_height(text, &note.replies),
            })
        })
        .collect()
}

fn encode_quads(quads: &[Quad]) -> String {
    quads
        .iter()
        .map(|quad| format!("{},{},{},{}", quad.x, quad.y, quad.w, quad.h))
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_quads(text: &str) -> Vec<Quad> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split(';').filter_map(decode_quad).collect()
}

fn decode_quad(text: &str) -> Option<Quad> {
    let mut parts = text.split(',');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let w = parts.next()?.parse().ok()?;
    let h = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Quad { x, y, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(top: f64, right: f64) -> LineBox {
        LineBox {
            top,
            height: 28.0,
            highlight_right: right,
            text_right: right,
        }
    }

    fn card(id: i64, top: f64, right: f64) -> CardIn {
        CardIn {
            id,
            line: line(top, right),
            collapsed_h: 52.0,
            hover_h: 80.0,
            selected_h: 120.0,
        }
    }

    fn focus_none() -> Focus {
        Focus {
            hover: None,
            selected: None,
        }
    }

    #[test]
    fn a_wrapped_highlight_uses_the_first_line_end() {
        let quads = [
            Quad {
                x: 972.0,
                y: 280.0,
                w: 164.0,
                h: 20.0,
            },
            Quad {
                x: 56.0,
                y: 308.0,
                w: 36.0,
                h: 20.0,
            },
        ];
        let line = first_line(&quads, 1136.0).unwrap();
        assert!((line.top - 280.0).abs() < 0.01);
        assert!((line.highlight_right - 1136.0).abs() < 0.01);
        assert!((connector_x(line.highlight_right, line.text_right) - 1136.0).abs() < 0.01);
        assert!(line.highlight_right > 200.0);
    }

    #[test]
    fn following_glyphs_on_the_same_line_push_the_start_past_the_words() {
        let quads = [Quad {
            x: 94.0,
            y: 140.0,
            w: 205.0,
            h: 20.0,
        }];
        let line = first_line(&quads, 1110.0).unwrap();
        assert!((line.highlight_right - 299.0).abs() < 0.01);
        assert!((line.text_right - 1110.0).abs() < 0.01);
    }

    #[test]
    fn rest_has_no_connector_and_cards_share_their_tops_with_the_lines() {
        let scene = place(
            &[card(1, 100.0, 400.0), card(2, 200.0, 400.0)],
            focus_none(),
            900.0,
            1200.0,
            0.0,
        );
        assert!(scene.cards.iter().all(|card| card.segments.is_empty()));
        assert!((scene.cards[0].y - 100.0).abs() < 0.01);
        assert!((scene.cards[1].y - 200.0).abs() < 0.01);
        assert!((scene.cards[0].x - 1216.0).abs() < 0.01);
        assert!((scene.cards[0].h - 52.0).abs() < 0.01);
    }

    #[test]
    fn crowded_cards_leave_eight_points_and_the_selected_card_returns() {
        let cards = [
            card(1, 100.0, 400.0),
            card(2, 110.0, 400.0),
            card(3, 120.0, 400.0),
        ];
        let scene = place(
            &cards,
            Focus {
                hover: None,
                selected: Some(2),
            },
            900.0,
            1200.0,
            0.0,
        );
        let selected = scene.cards.iter().find(|card| card.id == 2).unwrap();
        assert!((selected.y - 110.0).abs() < 0.01);
        assert!(selected.open);
        assert!(!selected.segments.is_empty());
        let above = scene.cards.iter().find(|card| card.id == 1).unwrap();
        assert!((above.y + above.h + GAP - selected.y).abs() < 0.01);
        let below = scene.cards.iter().find(|card| card.id == 3).unwrap();
        assert!(below.y >= selected.y + selected.h + GAP - 0.01);
    }

    #[test]
    fn a_selected_connector_starts_at_the_line_end_and_does_not_run_left() {
        let mut card = card(1, 280.0, 1136.0);
        card.line.text_right = 1136.0;
        let scene = place(
            &[card],
            Focus {
                hover: None,
                selected: Some(1),
            },
            900.0,
            1200.0,
            0.0,
        );
        let seg = &scene.cards[0].segments[0];
        assert!((seg.x - 1136.0).abs() < 0.01);
        assert!(seg.x > 200.0);
        assert!(!seg.fade);
        assert!((seg.y + 0.5 - (280.0 + 14.0)).abs() < 0.01);
    }

    #[test]
    fn hover_is_faded_and_collapsed_until_then() {
        let scene = place(
            &[card(1, 140.0, 1110.0)],
            Focus {
                hover: Some(1),
                selected: None,
            },
            900.0,
            1200.0,
            0.0,
        );
        assert!(scene.cards[0].open);
        assert!((scene.cards[0].h - 80.0).abs() < 0.01);
        assert!(scene.cards[0].segments[0].fade);
    }

    #[test]
    fn scrolling_off_the_line_hides_the_connector_and_scrolling_back_restores_it() {
        let mut tall = card(1, 100.0, 400.0);
        tall.selected_h = 900.0;
        let cards = [tall];
        let away = place(
            &cards,
            Focus {
                hover: None,
                selected: Some(1),
            },
            900.0,
            1200.0,
            40.0,
        );
        assert!(away.cards[0].segments.is_empty());
        assert!((away.cards[0].y - 60.0).abs() < 0.01);
        let back = place(
            &cards,
            Focus {
                hover: None,
                selected: Some(1),
            },
            900.0,
            1200.0,
            0.0,
        );
        assert!(!back.cards[0].segments.is_empty());
        assert!((back.cards[0].y - 100.0).abs() < 0.01);
    }

    #[test]
    fn a_card_taller_than_the_band_stays_above_the_composer_and_its_end_is_reachable() {
        let mut tall = card(1, 100.0, 400.0);
        tall.selected_h = 2000.0;
        let scene = place(
            &[tall],
            Focus {
                hover: None,
                selected: Some(1),
            },
            900.0,
            1200.0,
            f64::MAX,
        );
        let bottom = band_bottom(900.0);
        assert!((scene.scroll - scene.scroll_max).abs() < 0.01);
        let card = &scene.cards[0];
        assert!((card.y + card.h - (bottom - GAP)).abs() < 0.01);
        assert!(card.y + card.h <= bottom + 0.01);
        assert!(card.h > bottom - BAND_TOP);
    }

    #[test]
    fn a_line_in_the_composer_band_parks_the_card_eight_points_above_it() {
        let bottom = band_bottom(528.0);
        let scene = place(
            &[card(1, bottom + 4.0, 200.0)],
            focus_none(),
            528.0,
            338.0,
            0.0,
        );
        let card = &scene.cards[0];
        assert!((card.y + card.h - (bottom - GAP)).abs() < 0.01);
        assert!(card.segments.is_empty());
    }

    #[test]
    fn a_line_in_the_header_parks_the_card_at_the_band_top() {
        let scene = place(&[card(1, 10.0, 200.0)], focus_none(), 900.0, 1200.0, 0.0);
        assert!((scene.cards[0].y - BAND_TOP).abs() < 0.01);
    }

    #[test]
    fn a_turn_keeps_the_departing_page_until_it_lands() {
        let mut hold = Hold {
            book: "a.epub".into(),
            page: 3,
            scroll: 20.0,
            selected: Some(7),
            ..Hold::new()
        };
        let moving = LivePage {
            book: "a.epub".into(),
            spine: String::new(),
            page: 4,
            page_w: 600.0,
            page_h: 800.0,
            moving: true,
        };
        hold = reconcile(&hold, &moving);
        assert_eq!(hold.page, 3);
        assert!(hold.moving);
        assert!((hold.scroll - 20.0).abs() < 0.01);
        let landed = LivePage {
            moving: false,
            page: 4,
            ..moving
        };
        hold = reconcile(&hold, &landed);
        assert_eq!(hold.page, 4);
        assert!(!hold.moving);
        assert_eq!(hold.scroll, 0.0);
        assert_eq!(hold.selected, None);
    }

    #[test]
    fn another_book_does_not_keep_the_previous_page() {
        let hold = Hold {
            book: "a.epub".into(),
            page: 3,
            selected: Some(1),
            ..Hold::new()
        };
        let next = reconcile(
            &hold,
            &LivePage {
                book: "b.pdf".into(),
                spine: String::new(),
                page: 1,
                page_w: 100.0,
                page_h: 100.0,
                moving: false,
            },
        );
        assert_eq!(next.book, "b.pdf");
        assert_eq!(next.page, 1);
        assert_eq!(next.selected, None);
    }

    #[test]
    fn the_same_column_in_another_chapter_stays_on_its_spine() {
        let path = std::env::temp_dir().join(format!("margin-spine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join("annotations.sqlite");
        let live = LiveBook {
            book: "/books/a.epub".into(),
            gen: 3,
        };
        let mut one = sample_selection();
        one.book = live.book.clone();
        one.gen = live.gen;
        one.page = 1;
        one.spine = "ch1.xhtml".into();
        one.text = "甲".into();
        one.start = 0;
        one.end = 1;
        let mut two = one.clone();
        two.spine = "ch2.xhtml".into();
        two.text = "乙".into();
        let first = passage_from(one).unwrap();
        let second = passage_from(two).unwrap();
        let saved = on_return(
            ReturnKind::Key,
            &field_of(first.clone(), "一"),
            "一",
            &shown(&live, &first),
            &file,
        );
        assert!(saved.body.is_empty());
        let saved = on_return(
            ReturnKind::Key,
            &field_of(second.clone(), "二"),
            "二",
            &shown(&live, &second),
            &file,
        );
        assert!(saved.body.is_empty());
        let notes = Notes::open(&file).unwrap();
        let chapter = notes.list(&live.book, 1, "ch1.xhtml").unwrap();
        assert_eq!(chapter.len(), 1);
        assert_eq!(chapter[0].body, "一");
        assert_eq!(chapter[0].passage, "甲");
        let other = notes.list(&live.book, 1, "ch2.xhtml").unwrap();
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].body, "二");
        drop(notes);
        let reopened = Notes::open(&file).unwrap();
        assert_eq!(
            reopened.list(&live.book, 1, "ch1.xhtml").unwrap()[0].body,
            "一"
        );
        let hold = Hold {
            book: live.book.clone(),
            spine: "ch1.xhtml".into(),
            page: 1,
            selected: Some(chapter[0].id),
            ..Hold::new()
        };
        let next = reconcile(
            &hold,
            &LivePage {
                book: live.book,
                spine: "ch2.xhtml".into(),
                page: 1,
                page_w: 400.0,
                page_h: 600.0,
                moving: false,
            },
        );
        assert_eq!(next.spine, "ch2.xhtml");
        assert_eq!(next.page, 1);
        assert_eq!(next.selected, None);
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn notes_stay_inside_one_book_and_survive_reopen() {
        let dir = std::env::temp_dir().join(format!("margin-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("annotations.sqlite");
        let id = {
            let notes = Notes::open(&path).unwrap();
            let id = notes
                .insert(&NewNote {
                    book: "a.epub".into(),
                    page: 2,
                    spine: "chapter.xhtml".into(),
                    start: 10,
                    end: 24,
                    passage: "four or five".into(),
                    body: "a reader note".into(),
                    author: "读者".into(),
                    quads: vec![Quad {
                        x: 10.0,
                        y: 20.0,
                        w: 30.0,
                        h: 8.0,
                    }],
                    text_right: 80.0,
                })
                .unwrap();
            notes.reply(id, "a stored reply").unwrap();
            notes
                .insert(&NewNote {
                    book: "b.epub".into(),
                    page: 2,
                    spine: String::new(),
                    start: 0,
                    end: 4,
                    passage: "other".into(),
                    body: "other book".into(),
                    author: "读者".into(),
                    quads: vec![],
                    text_right: 0.0,
                })
                .unwrap();
            assert!(notes.list("a.epub", 1, "chapter.xhtml").unwrap().is_empty());
            assert_eq!(notes.list("a.epub", 2, "chapter.xhtml").unwrap().len(), 1);
            assert!(notes.list("a.epub", 2, "other.xhtml").unwrap().is_empty());
            assert_eq!(notes.list("b.epub", 2, "").unwrap()[0].body, "other book");
            id
        };
        let notes = Notes::open(&path).unwrap();
        let stored = notes.list("a.epub", 2, "chapter.xhtml").unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, id);
        assert_eq!(stored[0].replies, vec!["a stored reply".to_string()]);
        assert!((stored[0].quads[0].x - 10.0).abs() < 0.01);
        assert!(notes.reply(id + 50, "missing").is_err());
        assert!(notes
            .insert(&NewNote {
                book: String::new(),
                page: 1,
                spine: String::new(),
                start: 0,
                end: 0,
                passage: String::new(),
                body: String::new(),
                author: String::new(),
                quads: vec![],
                text_right: 0.0,
            })
            .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mapped_quads_follow_the_page_fit_and_a_zero_scale_draws_nothing() {
        let notes = [StoredNote {
            id: 4,
            page: 1,
            spine: String::new(),
            start: 0,
            end: 3,
            passage: "ink".into(),
            body: "note".into(),
            author: "读者".into(),
            quads: vec![Quad {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 8.0,
            }],
            text_right: 90.0,
            replies: vec!["reply".into()],
        }];
        let fit = Fit {
            origin_x: 100.0,
            origin_y: 40.0,
            scale: 2.0,
        };
        let cards = cards_from(&notes, fit);
        assert_eq!(cards.len(), 1);
        assert!((cards[0].line.top - 80.0).abs() < 0.01);
        assert!((cards[0].line.highlight_right - 180.0).abs() < 0.01);
        assert!((cards[0].line.text_right - 280.0).abs() < 0.01);
        assert!(cards[0].selected_h > cards[0].hover_h);
        assert!(cards_from(&notes, Fit { scale: 0.0, ..fit }).is_empty());
    }

    #[test]
    fn an_empty_body_measures_the_passage() {
        let passage = "word ".repeat(80);
        let notes = [StoredNote {
            id: 1,
            page: 1,
            spine: String::new(),
            start: 0,
            end: 4,
            passage: passage.clone(),
            body: String::new(),
            author: "读者".into(),
            quads: vec![Quad {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 8.0,
            }],
            text_right: 40.0,
            replies: Vec::new(),
        }];
        let cards = cards_from(
            &notes,
            Fit {
                origin_x: 0.0,
                origin_y: 0.0,
                scale: 1.0,
            },
        );
        assert_eq!(cards.len(), 1);
        assert!((cards[0].hover_h - open_height(&passage, &[])).abs() < 0.01);
        assert!(cards[0].hover_h > open_height("", &[]));
    }

    #[test]
    fn an_open_card_keeps_a_spare_line() {
        let tight = PAD_Y + LINE_H + LINE_H + PAD_Y;
        assert!(open_height("a", &[]) >= tight + LINE_H);
    }

    #[test]
    fn a_note_without_quads_draws_nothing() {
        let notes = [StoredNote {
            id: 1,
            page: 1,
            spine: String::new(),
            start: 0,
            end: 0,
            passage: "a".into(),
            body: "b".into(),
            author: "读者".into(),
            quads: Vec::new(),
            text_right: 10.0,
            replies: Vec::new(),
        }];
        let fit = Fit {
            origin_x: 0.0,
            origin_y: 0.0,
            scale: 1.0,
        };
        assert!(cards_from(&notes, fit).is_empty());
    }

    #[test]
    fn a_selected_card_in_the_header_returns_to_the_band_top() {
        let scene = place(
            &[card(1, 10.0, 200.0), card(2, 20.0, 200.0)],
            Focus {
                hover: None,
                selected: Some(2),
            },
            900.0,
            1200.0,
            0.0,
        );
        let selected = scene.cards.iter().find(|card| card.id == 2).unwrap();
        assert!((selected.y - BAND_TOP).abs() < 0.01);
        assert!(selected.segments.iter().all(|seg| seg.x + 0.01 >= 200.0));
    }

    #[test]
    fn a_selected_card_in_the_composer_band_returns_above_it() {
        let bottom = band_bottom(528.0);
        let mut tall = card(1, 80.0, 200.0);
        tall.collapsed_h = 400.0;
        let scene = place(
            &[tall, card(2, bottom + 4.0, 200.0)],
            Focus {
                hover: None,
                selected: Some(2),
            },
            528.0,
            338.0,
            0.0,
        );
        let selected = scene.cards.iter().find(|card| card.id == 2).unwrap();
        assert!((selected.y + selected.h - (bottom - GAP)).abs() < 0.01);
        assert!(selected.y + selected.h <= bottom + 0.01);
    }

    #[test]
    fn a_reopened_wrap_keeps_the_first_line() {
        let path = std::env::temp_dir().join(format!("margin-wrap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join("annotations.sqlite");
        let notes = Notes::open(&file).unwrap();
        notes
            .insert(&NewNote {
                book: "book".into(),
                page: 1,
                spine: String::new(),
                start: 0,
                end: 8,
                passage: "four or five thousand a year".into(),
                body: "note".into(),
                author: "读者".into(),
                quads: vec![
                    Quad {
                        x: 972.0,
                        y: 280.0,
                        w: 164.0,
                        h: 20.0,
                    },
                    Quad {
                        x: 56.0,
                        y: 308.0,
                        w: 36.0,
                        h: 20.0,
                    },
                ],
                text_right: 1136.0,
            })
            .unwrap();
        drop(notes);
        let notes = Notes::open(&file).unwrap();
        let stored = notes.list("book", 1, "").unwrap();
        let cards = cards_from(
            &stored,
            Fit {
                origin_x: 0.0,
                origin_y: 0.0,
                scale: 1.0,
            },
        );
        assert_eq!(cards.len(), 1);
        assert!((cards[0].line.highlight_right - 1136.0).abs() < 0.01);
        assert!((cards[0].line.top - 280.0).abs() < 0.01);
        let scene = place(
            &cards,
            Focus {
                hover: None,
                selected: Some(stored[0].id),
            },
            900.0,
            1400.0,
            0.0,
        );
        let seg = &scene.cards[0].segments[0];
        assert!((seg.x - 1136.0).abs() < 0.01);
        assert!(seg.x > 56.0);
        let _ = std::fs::remove_dir_all(path);
    }

    fn sample_selection() -> Selection {
        Selection {
            book: "/books/a.pdf".into(),
            gen: 4,
            page: 3,
            spine: String::new(),
            start: 0,
            end: 2,
            text: "你😀".into(),
            quads: vec![Quad {
                x: 12.0,
                y: 30.0,
                w: 40.0,
                h: 16.0,
            }],
            text_right: 80.0,
        }
    }

    fn shown(live: &LiveBook, passage: &Passage) -> Visible {
        Visible {
            book: live.clone(),
            page: passage.page,
            spine: passage.spine.clone(),
            moving: false,
        }
    }

    fn field_of(passage: Passage, body: &str) -> NoteField {
        NoteField {
            body: body.into(),
            passage: Some(passage),
            ..NoteField::default()
        }
    }

    #[test]
    fn pdf_line_end_stops_at_the_first_line() {
        let glyphs = [
            GlyphBox {
                index: 0,
                x: 10.0,
                y: 20.0,
                w: 8.0,
                h: 10.0,
            },
            GlyphBox {
                index: 1,
                x: 20.0,
                y: 20.0,
                w: 8.0,
                h: 10.0,
            },
            GlyphBox {
                index: 2,
                x: 40.0,
                y: 20.0,
                w: 6.0,
                h: 10.0,
            },
            GlyphBox {
                index: 3,
                x: 10.0,
                y: 40.0,
                w: 30.0,
                h: 10.0,
            },
        ];
        let right = pdf_line_end(&glyphs, 0, 2).unwrap();
        assert!((right - 46.0).abs() < 0.01);
        assert!(pdf_line_end(&glyphs, 9, 10).is_none());
    }

    #[test]
    fn an_epub_measure_keeps_real_rects_and_rejects_a_utf16_span() {
        let quads = measured_quads(&[
            MeasuredRect {
                x: 12.0,
                y: 30.0,
                w: 40.0,
                h: 16.0,
            },
            MeasuredRect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 10.0,
            },
            MeasuredRect {
                x: 8.0,
                y: 50.0,
                w: 20.0,
                h: 16.0,
            },
        ]);
        assert_eq!(quads.len(), 2);
        let mut selection = sample_selection();
        selection.quads = quads;
        selection.spine = "chapter.xhtml".into();
        let passage = passage_from(selection.clone()).unwrap();
        assert_eq!(passage.end - passage.start, 2);
        assert_eq!(passage.quads.len(), 2);
        selection.end = 3;
        assert!(passage_from(selection).is_none());
        assert_eq!(epub_page_size(0.0, 800.0), (0.0, 0.0));
        assert_eq!(epub_page_size(640.0, 800.0), (640.0, 800.0));
    }

    #[test]
    fn return_saves_reopens_and_keeps_the_other_book_empty() {
        let path =
            std::env::temp_dir().join(format!("margin-return-{}-{}", std::process::id(), "save"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join("annotations.sqlite");
        let passage = passage_from(sample_selection()).unwrap();
        let live = LiveBook {
            book: passage.book.clone(),
            gen: passage.gen,
        };
        let saved = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "  一句  "),
            "  一句  ",
            &shown(&live, &passage),
            &file,
        );
        assert!(saved.body.is_empty());
        assert!(saved.notice.is_empty());
        assert_eq!(saved.fingerprint.as_ref().unwrap().body, "一句");
        let again = on_return(
            ReturnKind::Key,
            &saved,
            "  一句  ",
            &shown(&live, &passage),
            &file,
        );
        assert!(again.body.is_empty());
        let notes = Notes::open(&file).unwrap();
        let stored = notes
            .list(&passage.book, passage.page, &passage.spine)
            .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].body, "一句");
        assert_eq!(stored[0].passage, "你😀");
        assert_eq!(stored[0].start, 0);
        assert_eq!(stored[0].end, 2);
        assert!((stored[0].text_right - 80.0).abs() < 0.01);
        assert!(notes
            .list(&passage.book, 5, &passage.spine)
            .unwrap()
            .is_empty());
        assert!(notes
            .list("/books/b.epub", passage.page, &passage.spine)
            .unwrap()
            .is_empty());
        drop(notes);
        let reopened = Notes::open(&file).unwrap();
        let cards = cards_from(
            &reopened
                .list(&passage.book, passage.page, &passage.spine)
                .unwrap(),
            Fit {
                origin_x: 0.0,
                origin_y: 0.0,
                scale: 1.0,
            },
        );
        assert_eq!(cards.len(), 1);
        assert!((cards[0].line.top - 30.0).abs() < 0.01);
        assert!((cards[0].line.text_right - 80.0).abs() < 0.01);
        let typed = note_typed(&again, "一句");
        assert!(typed.fingerprint.is_none());
        let second = on_return(
            ReturnKind::Key,
            &typed,
            "一句",
            &shown(&live, &passage),
            &file,
        );
        assert!(second.body.is_empty());
        assert_eq!(
            reopened
                .list(&passage.book, passage.page, &passage.spine)
                .unwrap()
                .len(),
            2
        );
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn a_blank_missing_stale_composing_or_blur_keeps_the_draft() {
        let path =
            std::env::temp_dir().join(format!("margin-return-{}-{}", std::process::id(), "keep"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let file = path.join("annotations.sqlite");
        let passage = passage_from(sample_selection()).unwrap();
        let live = LiveBook {
            book: passage.book.clone(),
            gen: passage.gen,
        };
        let blank = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "   "),
            "   ",
            &shown(&live, &passage),
            &file,
        );
        assert_eq!(blank.body, "   ");
        assert_eq!(blank.notice, NOTICE_BLANK);
        let missing = on_return(
            ReturnKind::Key,
            &NoteField {
                body: "留下".into(),
                ..NoteField::default()
            },
            "留下",
            &shown(&live, &passage),
            &file,
        );
        assert_eq!(missing.body, "留下");
        assert_eq!(missing.notice, NOTICE_SELECT);
        let other = LiveBook {
            book: "/books/b.epub".into(),
            gen: live.gen,
        };
        let stale = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "摘录"),
            "摘录",
            &shown(&other, &passage),
            &file,
        );
        assert_eq!(stale.body, "摘录");
        assert_eq!(stale.notice, NOTICE_STALE);
        let reopened = LiveBook {
            book: live.book.clone(),
            gen: live.gen + 1,
        };
        let old_token = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "摘录"),
            "摘录",
            &shown(&reopened, &passage),
            &file,
        );
        assert_eq!(old_token.body, "摘录");
        assert_eq!(old_token.notice, NOTICE_STALE);
        assert!(take_passage(Some(passage.clone()), &reopened).is_none());
        let mut composing = field_of(passage.clone(), "ni");
        composing.composing = true;
        let held = on_return(
            ReturnKind::Key,
            &composing,
            "ni",
            &shown(&live, &passage),
            &file,
        );
        assert_eq!(held.body, "ni");
        assert!(held.notice.is_empty());
        let ready = field_of(passage.clone(), "ni");
        let blurred = on_return(
            ReturnKind::Blur,
            &ready,
            "ni",
            &shown(&live, &passage),
            &file,
        );
        assert_eq!(blurred.body, "ni");
        assert!(blurred.notice.is_empty());
        assert!(Notes::open(&file)
            .unwrap()
            .list(&live.book, passage.page, &passage.spine)
            .unwrap()
            .is_empty());
        let mut left = shown(&live, &passage);
        left.page += 1;
        let departed = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "留下"),
            "留下",
            &left,
            &file,
        );
        assert_eq!(departed.body, "留下");
        assert_eq!(departed.notice, NOTICE_STALE);
        let mut other_chapter = shown(&live, &passage);
        other_chapter.spine = "other.xhtml".into();
        let jumped = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "留下"),
            "留下",
            &other_chapter,
            &file,
        );
        assert_eq!(jumped.body, "留下");
        assert_eq!(jumped.notice, NOTICE_STALE);
        let mut turning = shown(&live, &passage);
        turning.moving = true;
        let kept_page = passage.page;
        let kept_spine = passage.spine.clone();
        let mid = on_return(
            ReturnKind::Key,
            &field_of(passage, "留下"),
            "留下",
            &turning,
            &file,
        );
        assert_eq!(mid.body, "留下");
        assert_eq!(mid.notice, NOTICE_STALE);
        assert!(Notes::open(&file)
            .unwrap()
            .list(&live.book, kept_page, &kept_spine)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn an_epub_card_uses_the_fresh_line_and_not_the_stored_box() {
        let stored = StoredNote {
            id: 4,
            page: 1,
            spine: "ch1.xhtml".into(),
            start: 0,
            end: 2,
            passage: "甲".into(),
            body: "一".into(),
            author: "读者".into(),
            quads: vec![Quad {
                x: 1.0,
                y: 1.0,
                w: 8.0,
                h: 8.0,
            }],
            text_right: 20.0,
            replies: Vec::new(),
        };
        let fresh = FreshMeasure {
            id: 4,
            page: 3,
            quads: vec![Quad {
                x: 90.0,
                y: 40.0,
                w: 24.0,
                h: 16.0,
            }],
            text_right: 140.0,
        };
        assert!(epub_shown(
            std::slice::from_ref(&stored),
            std::slice::from_ref(&fresh),
            1
        )
        .is_empty());
        assert!(epub_shown(std::slice::from_ref(&stored), &[], 3).is_empty());
        let shown = epub_shown(
            std::slice::from_ref(&stored),
            std::slice::from_ref(&fresh),
            3,
        );
        assert_eq!(shown.len(), 1);
        assert!((shown[0].quads[0].x - 90.0).abs() < 0.01);
        assert!((shown[0].text_right - 140.0).abs() < 0.01);
        assert_eq!(shown[0].page, 3);
    }

    #[test]
    fn a_failed_store_keeps_the_draft() {
        let path =
            std::env::temp_dir().join(format!("margin-return-{}-{}", std::process::id(), "fail"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let blocker = path.join("blocked");
        std::fs::write(&blocker, b"x").unwrap();
        let passage = passage_from(sample_selection()).unwrap();
        let live = LiveBook {
            book: passage.book.clone(),
            gen: passage.gen,
        };
        let failed = on_return(
            ReturnKind::Key,
            &field_of(passage.clone(), "别丢"),
            "别丢",
            &shown(&live, &passage),
            &blocker.join("annotations.sqlite"),
        );
        assert_eq!(failed.body, "别丢");
        assert!(!failed.notice.is_empty());
        assert!(failed.fingerprint.is_none());
        let _ = std::fs::remove_dir_all(path);
    }
}
