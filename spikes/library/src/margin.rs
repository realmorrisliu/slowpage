//! Annotation cards. Geometry is the 2026-11-20 22:42 set. Storage is the
//! local SQLite the route already assigned to SuperReader. This module does
//! not call an agent and does not ship sample notes.

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
    pub page: u32,
    pub page_w: f64,
    pub page_h: f64,
    pub moving: bool,
}

impl Default for LivePage {
    fn default() -> Self {
        Self {
            book: String::new(),
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
    if hold.book != live.book {
        return Hold {
            book: live.book.clone(),
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
        self.conn
            .execute(
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
        Ok(self.conn.last_insert_rowid())
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

    pub fn list(&self, book: &str, page: u32) -> Result<Vec<StoredNote>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, page, spine, start_at, end_at, passage, body, author, quads, text_right
                 FROM notes WHERE book = ?1 AND page = ?2 ORDER BY id",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![book, page], |row| {
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
            Some(CardIn {
                id: note.id,
                line,
                collapsed_h: collapsed_height(),
                hover_h: open_height(&note.body, &[]),
                selected_h: open_height(&note.body, &note.replies),
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
            assert!(notes.list("a.epub", 1).unwrap().is_empty());
            assert_eq!(notes.list("a.epub", 2).unwrap().len(), 1);
            assert_eq!(notes.list("b.epub", 2).unwrap()[0].body, "other book");
            id
        };
        let notes = Notes::open(&path).unwrap();
        let stored = notes.list("a.epub", 2).unwrap();
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
        let stored = notes.list("book", 1).unwrap();
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
}
