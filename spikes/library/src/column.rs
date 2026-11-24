//! Bookmarks and in-book search for one open book.
//!
//! The margin column has three modes. The page frame is not one of them.
//! Bookmarks live in the same SQLite file as notes. A search walks units the
//! caller feeds one at a time. A keystroke does not scan the book.

use std::cell::Cell;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::margin::{park_card, release_composition, CardKind, Face, LivePage, MarginInk};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Notes,
    Marks,
    Find,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    Chars,
    Utf16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Searching,
    Done,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub href: String,
    pub page: u32,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub id: i64,
    pub book: String,
    pub page: u32,
    pub href: String,
    pub offset: u32,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub page: u32,
    pub href: String,
    pub start: u32,
    pub end: u32,
    pub kind: IndexKind,
    pub snippet: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeatKind {
    New,
    Card { note_id: i64, kind: CardKind },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSeat {
    pub book: String,
    pub page: u32,
    pub href: String,
    pub kind: SeatKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Find {
    pub draft: String,
    pub query: String,
    pub composing: bool,
    pub generation: u64,
    pub hits: Vec<Hit>,
    pub index: Option<usize>,
    pub phase: Phase,
    pub seen: u32,
    pub total: u32,
    pub text_units: u32,
    pub empty_units: u32,
    pub failed_units: u32,
    pub pending: bool,
    /// The reading place the automatic landing is still owed against.
    /// A scan does not retarget this after the reader moves.
    pub origin: Place,
    /// Navigation epoch caught when this search began to owe a landing.
    /// A newer epoch means the reader has started a move, even when the
    /// place report still shows the origin.
    pub nav: u64,
    pub navigated: bool,
    pub seat: Option<EditorSeat>,
    pub focus: u64,
}

impl Default for Find {
    fn default() -> Self {
        Self {
            draft: String::new(),
            query: String::new(),
            composing: false,
            generation: 0,
            hits: Vec::new(),
            index: None,
            phase: Phase::Idle,
            seen: 0,
            total: 0,
            text_units: 0,
            empty_units: 0,
            failed_units: 0,
            pending: false,
            origin: Place {
                href: String::new(),
                page: 0,
                offset: 0,
            },
            nav: 0,
            navigated: false,
            seat: None,
            focus: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub mode: Mode,
    pub mode_before: Mode,
    pub book: String,
    /// Spine hrefs in document order. Empty for a PDF, whose page number is
    /// already the document unit.
    pub order: Vec<String>,
    pub marks: Vec<Bookmark>,
    pub undo: Option<Bookmark>,
    pub undo_until: u64,
    pub find: Find,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            mode: Mode::Notes,
            mode_before: Mode::Notes,
            book: String::new(),
            order: Vec::new(),
            marks: Vec::new(),
            undo: None,
            undo_until: 0,
            find: Find::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindLine {
    pub text: String,
    pub retry: bool,
}

pub struct Marks {
    conn: Connection,
}

impl Marks {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
        }
        let conn = Connection::open(path).map_err(|err| err.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY,
                book TEXT NOT NULL,
                page INTEGER NOT NULL,
                href TEXT NOT NULL,
                offset INTEGER NOT NULL,
                label TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS bookmarks_book ON bookmarks(book);",
        )
        .map_err(|err| err.to_string())?;
        Ok(Self { conn })
    }

    pub fn list(&self, book: &str) -> Result<Vec<Bookmark>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, book, page, href, offset, label
                 FROM bookmarks WHERE book = ?1
                 ORDER BY page, offset, id",
            )
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map(params![book], read_mark)
            .map_err(|err| err.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|err| err.to_string())
    }

    pub fn at(
        &self,
        book: &str,
        page: u32,
        href: &str,
        offset: u32,
    ) -> Result<Option<Bookmark>, String> {
        let rows = self.list(book)?;
        Ok(rows
            .into_iter()
            .find(|mark| same_place(mark, page, href, offset)))
    }

    pub fn insert(
        &self,
        book: &str,
        page: u32,
        href: &str,
        offset: u32,
        label: &str,
    ) -> Result<Bookmark, String> {
        let label = if label.is_empty() {
            bookmark_label(
                &Place {
                    href: href.to_string(),
                    page,
                    offset,
                },
                "",
            )
        } else {
            label.to_string()
        };
        self.conn
            .execute(
                "INSERT INTO bookmarks (book, page, href, offset, label) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![book, page, href, offset, label],
            )
            .map_err(|err| err.to_string())?;
        let id = self.conn.last_insert_rowid();
        Ok(Bookmark {
            id,
            book: book.to_string(),
            page,
            href: href.to_string(),
            offset,
            label,
        })
    }

    pub fn remove(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

fn read_mark(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bookmark> {
    Ok(Bookmark {
        id: row.get(0)?,
        book: row.get(1)?,
        page: row.get(2)?,
        href: row.get(3)?,
        offset: row.get(4)?,
        label: row.get(5)?,
    })
}

pub fn reload(column: &Column, marks: Vec<Bookmark>) -> Column {
    let mut next = column.clone();
    next.marks = marks;
    next
}

pub fn expire_undo(column: &Column, now_ms: u64) -> Column {
    if column.undo.is_some() && now_ms >= column.undo_until {
        let mut next = column.clone();
        next.undo = None;
        next.undo_until = 0;
        return next;
    }
    column.clone()
}

pub const UNDO_MS: u64 = 4_000;

pub fn toggle_mark(
    column: &Column,
    marks: &Marks,
    place: &Place,
    label: &str,
    now_ms: u64,
) -> Result<Column, String> {
    if column.book.is_empty() {
        return Ok(column.clone());
    }
    let mut next = column.clone();
    if let Some(found) = marks.at(&column.book, place.page, &place.href, place.offset)? {
        marks.remove(found.id)?;
        next.undo = Some(found);
        next.undo_until = now_ms.saturating_add(UNDO_MS);
    } else {
        marks.insert(&column.book, place.page, &place.href, place.offset, label)?;
        next.undo = None;
        next.undo_until = 0;
    }
    next.marks = marks.list(&column.book)?;
    Ok(next)
}

pub fn delete_mark(column: &Column, marks: &Marks, id: i64, now_ms: u64) -> Result<Column, String> {
    let Some(found) = column.marks.iter().find(|mark| mark.id == id).cloned() else {
        return Ok(column.clone());
    };
    marks.remove(id)?;
    let mut next = column.clone();
    next.undo = Some(found);
    next.undo_until = now_ms.saturating_add(UNDO_MS);
    next.marks = marks.list(&column.book)?;
    Ok(next)
}

pub fn undo_delete(column: &Column, marks: &Marks) -> Result<Column, String> {
    let Some(mark) = column.undo.clone() else {
        return Ok(column.clone());
    };
    let _ = marks.insert(&mark.book, mark.page, &mark.href, mark.offset, &mark.label)?;
    let mut next = column.clone();
    next.undo = None;
    next.undo_until = 0;
    next.marks = marks.list(&column.book)?;
    Ok(next)
}

pub fn enter_marks(column: &Column, ink: &MarginInk) -> (Column, MarginInk) {
    let mut next = column.clone();
    if next.mode != Mode::Find {
        next.mode_before = next.mode;
    }
    next.mode = Mode::Marks;
    (next, park_card(&release_composition(ink)))
}

pub fn enter_notes(column: &Column) -> Column {
    let mut next = column.clone();
    next.mode = Mode::Notes;
    next
}

pub fn enter_find(column: &Column, ink: &MarginInk, live: &LivePage) -> (Column, MarginInk) {
    let mut next = column.clone();
    if next.mode != Mode::Find {
        next.mode_before = next.mode;
        next.find.navigated = false;
        next.find.seat = seat_of(ink, live);
    }
    next.mode = Mode::Find;
    next.find.focus = next.find.focus.saturating_add(1);
    (next, release_composition(ink))
}

fn seat_of(ink: &MarginInk, live: &LivePage) -> Option<EditorSeat> {
    if live.book.is_empty() {
        return None;
    }
    let kind = match &ink.face {
        Face::Card(key) if key.book == live.book => SeatKind::Card {
            note_id: key.note_id,
            kind: key.kind,
        },
        Face::New
            if ink.new_note.composing
                || !ink.new_note.body.trim().is_empty()
                || ink.focus_new > 0 =>
        {
            SeatKind::New
        }
        _ => return None,
    };
    Some(EditorSeat {
        book: live.book.clone(),
        page: live.page,
        href: live.spine.clone(),
        kind,
    })
}

pub struct ClosedFind {
    pub column: Column,
    pub seat: Option<EditorSeat>,
}

pub fn close_find(column: &Column) -> ClosedFind {
    let mut next = column.clone();
    let seat = if next.find.navigated {
        None
    } else {
        next.find.seat.clone()
    };
    let back = if next.mode_before == Mode::Find {
        Mode::Notes
    } else {
        next.mode_before
    };
    next.mode = back;
    next.find.hits.clear();
    next.find.index = None;
    next.find.pending = false;
    next.find.phase = Phase::Idle;
    next.find.seen = 0;
    next.find.total = 0;
    next.find.text_units = 0;
    next.find.empty_units = 0;
    next.find.failed_units = 0;
    ClosedFind { column: next, seat }
}

pub fn refocus_after_find(
    ink: &MarginInk,
    seat: Option<&EditorSeat>,
    live: &LivePage,
) -> MarginInk {
    let Some(seat) = seat else {
        return ink.clone();
    };
    if seat.book != live.book || seat.page != live.page || seat.href != live.spine {
        return ink.clone();
    }
    match (&seat.kind, &ink.face) {
        (SeatKind::New, Face::New) => {
            let mut next = ink.clone();
            next.focus_new = next.focus_new.saturating_add(1);
            next
        }
        (SeatKind::Card { note_id, kind }, Face::Card(key))
            if key.note_id == *note_id && key.kind == *kind && key.book == seat.book =>
        {
            ink.clone()
        }
        _ => ink.clone(),
    }
}

/// Esc puts the column back, then the caret. A card that is not the same
/// book, card, and page is parked. Its draft stays. The field is not built.
pub fn restore_after_find(
    column: &Column,
    ink: &MarginInk,
    seat: Option<&EditorSeat>,
    live: &LivePage,
) -> MarginInk {
    let focused = refocus_after_find(ink, seat, live);
    let landed = match (seat, &ink.face) {
        (Some(_), Face::New) if focused.focus_new > ink.focus_new => true,
        (Some(seat), Face::Card(key))
            if seat.book == live.book
                && seat.page == live.page
                && seat.href == live.spine
                && matches!(
                    &seat.kind,
                    SeatKind::Card { note_id, kind }
                        if *note_id == key.note_id && *kind == key.kind && key.book == seat.book
                ) =>
        {
            true
        }
        _ => false,
    };
    if landed {
        return focused;
    }
    if column.mode == Mode::Notes {
        park_card(ink)
    } else {
        ink.clone()
    }
}

pub fn clear_book(column: &Column, book: &str) -> Column {
    if column.book == book {
        return column.clone();
    }
    Column {
        book: book.to_string(),
        ..Column::default()
    }
}

pub fn type_find(column: &Column, draft: &str) -> Column {
    let mut next = column.clone();
    next.find.draft = draft.to_string();
    if next.find.composing {
        return next;
    }
    confirm_draft(&mut next);
    next
}

pub fn compose_find(column: &Column, composing: bool) -> Column {
    let mut next = column.clone();
    next.find.composing = composing;
    if composing {
        return next;
    }
    confirm_draft(&mut next);
    next
}

fn confirm_draft(column: &mut Column) {
    if column.find.draft == column.find.query {
        return;
    }
    let draft = column.find.draft.clone();
    begin_query(column, &draft);
}

fn begin_query(column: &mut Column, query: &str) {
    column.find.query = query.to_string();
    column.find.generation = column.find.generation.saturating_add(1);
    column.find.hits.clear();
    column.find.index = None;
    column.find.pending = false;
    column.find.seen = 0;
    column.find.total = 0;
    column.find.text_units = 0;
    column.find.empty_units = 0;
    column.find.failed_units = 0;
    column.find.phase = if query.trim().is_empty() {
        Phase::Idle
    } else {
        Phase::Searching
    };
}

pub fn retry_find(column: &Column) -> Column {
    let mut next = column.clone();
    if next.find.query.trim().is_empty() {
        return next;
    }
    let query = next.find.query.clone();
    next.find.draft = query.clone();
    begin_query(&mut next, &query);
    next
}

/// Return in the find field. A composition does not search or step.
/// A new query clears hits and waits for the first one. It does not move.
pub fn find_return(column: &Column, here: &Place) -> (Column, Option<Hit>) {
    if column.find.composing {
        return (column.clone(), None);
    }
    let mut next = column.clone();
    if next.find.draft != next.find.query {
        let draft = next.find.draft.clone();
        begin_query(&mut next, &draft);
        if next.find.phase == Phase::Searching {
            owe_landing(&mut next, here);
        }
        return (next, None);
    }
    if next.find.query.trim().is_empty() {
        return (next, None);
    }
    if next.find.hits.is_empty() {
        if next.find.phase != Phase::Searching {
            let query = next.find.query.clone();
            begin_query(&mut next, &query);
        }
        owe_landing(&mut next, here);
        return (next, None);
    }
    step(&next, true, here)
}

fn scan_finished(find: &Find) -> bool {
    matches!(find.phase, Phase::Done | Phase::Incomplete)
        || (find.total > 0 && find.seen >= find.total)
}

pub fn step(column: &Column, forward: bool, here: &Place) -> (Column, Option<Hit>) {
    if column.find.composing || column.find.hits.is_empty() {
        return (column.clone(), None);
    }
    let finished = scan_finished(&column.find);
    let len = column.find.hits.len();
    let index = match column.find.index {
        Some(current) if forward && current + 1 < len => current + 1,
        Some(_) if forward && finished => 0,
        Some(_) if forward => return (column.clone(), None),
        Some(0) if finished => len - 1,
        Some(0) => return (column.clone(), None),
        Some(current) => current - 1,
        None if forward => {
            let Some(index) = pending_hit(&column.order, &column.find.hits, here, finished) else {
                return (column.clone(), None);
            };
            index
        }
        None => match last_covered(&column.order, &column.find.hits, here) {
            Some(index) => index,
            None if finished => len - 1,
            None => return (column.clone(), None),
        },
    };
    let mut next = column.clone();
    next.find.index = Some(index);
    next.find.pending = false;
    let hit = next.find.hits[index].clone();
    if !same_anchor(&hit, here) {
        next.find.navigated = true;
    }
    (next, Some(hit))
}

/// Spine order, then the character offset. A PDF has an empty href, so its
/// page number is the unit. An EPUB column page is not a spine index.
fn document_key(order: &[String], href: &str, page: u32, offset: u32) -> (u32, u32) {
    if href.is_empty() {
        return (page, offset);
    }
    let want = spike_epub::normalize_href(href);
    let unit = order
        .iter()
        .position(|item| spike_epub::normalize_href(item) == want)
        .map(|index| index as u32 + 1)
        .unwrap_or(u32::MAX);
    (unit, offset)
}

fn hit_key(hit: &Hit) -> (u32, u32) {
    (hit.page, hit.start)
}

fn first_from(order: &[String], hits: &[Hit], here: &Place) -> usize {
    let here_key = document_key(order, &here.href, here.page, here.offset);
    hits.iter()
        .position(|hit| hit_key(hit) >= here_key)
        .unwrap_or(0)
}

fn last_covered(order: &[String], hits: &[Hit], here: &Place) -> Option<usize> {
    let here_key = document_key(order, &here.href, here.page, here.offset);
    hits.iter().rposition(|hit| hit_key(hit) < here_key)
}

/// The scan's first jump. While earlier units are the only hits, this stays
/// empty so a later chapter is not pulled back to chapter 1. A finished scan
/// wraps.
pub fn pending_hit(order: &[String], hits: &[Hit], here: &Place, finished: bool) -> Option<usize> {
    if hits.is_empty() {
        return None;
    }
    let here_key = document_key(order, &here.href, here.page, here.offset);
    if hits.iter().any(|hit| hit_key(hit) >= here_key) || finished {
        Some(first_from(order, hits, here))
    } else {
        None
    }
}

fn same_anchor(hit: &Hit, here: &Place) -> bool {
    if here.href.is_empty() {
        hit.href.is_empty() && hit.page == here.page && hit.start == here.offset
    } else {
        spike_epub::normalize_href(&hit.href) == spike_epub::normalize_href(&here.href)
            && hit.start == here.offset
    }
}

pub fn hit_place(hit: &Hit) -> Place {
    Place {
        href: hit.href.clone(),
        page: hit.page,
        offset: hit.start,
    }
}

/// The reader left the place the first landing was owed against.
/// An EPUB page number is a reflow result, so only the spine href and the
/// UTF-16 offset count. A PDF has no href, so its page number is the place.
fn reader_left(origin: &Place, live: &Place) -> bool {
    if origin.href.is_empty() && live.href.is_empty() {
        origin.page != live.page
    } else {
        spike_epub::normalize_href(&origin.href) != spike_epub::normalize_href(&live.href)
            || origin.offset != live.offset
    }
}

/// A page turn, a bookmark, a contents jump, or an in-book link. The scan
/// keeps the hits it already has and no longer owes a landing.
///
/// Callers that already hold the column use this. A move whose place is
/// written back later uses [`arm_nav`] instead, and the next snapshot
/// carries that epoch.
pub fn release_landing(column: &Column) -> Column {
    let mut next = column.clone();
    next.find.pending = false;
    next
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavIntent {
    Key,
    PageEdge,
    Trackpad,
    PdfGlide,
    Toc,
    Link,
    Bookmark,
}

thread_local! {
    static NAV_EPOCH: Cell<u64> = const { Cell::new(0) };
    static NAV_LAST: Cell<u8> = const { Cell::new(0) };
}

fn intent_tag(intent: NavIntent) -> u8 {
    match intent {
        NavIntent::Key => 1,
        NavIntent::PageEdge => 2,
        NavIntent::Trackpad => 3,
        NavIntent::PdfGlide => 4,
        NavIntent::Toc => 5,
        NavIntent::Link => 6,
        NavIntent::Bookmark => 7,
    }
}

pub fn nav_epoch() -> u64 {
    NAV_EPOCH.with(|cell| cell.get())
}

#[cfg(test)]
fn last_intent_tag() -> u8 {
    NAV_LAST.with(|cell| cell.get())
}

/// Record one explicit navigation before its asynchronous move.
/// The returned epoch is what the next snapshot must carry.
pub fn arm_nav(intent: NavIntent) -> u64 {
    NAV_LAST.with(|cell| cell.set(intent_tag(intent)));
    NAV_EPOCH.with(|cell| {
        let next = cell.get().saturating_add(1);
        cell.set(next);
        next
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageCommand {
    pub delta: i64,
    pub intent: Option<NavIntent>,
}

/// A page command arms only when it will move. Delta 0 is not a gesture.
/// The page edge is `edge`. Every other moving command is a key.
pub fn page_command(delta: i64, intent: Option<&str>) -> PageCommand {
    if delta == 0 {
        return PageCommand {
            delta: 0,
            intent: None,
        };
    }
    let intent = if intent == Some("edge") {
        NavIntent::PageEdge
    } else {
        NavIntent::Key
    };
    PageCommand {
        delta,
        intent: Some(intent),
    }
}

/// A column computed before a nested publish must not move the epoch backward.
/// The newer epoch already cancelled the owed landing.
pub fn observe_nav(current: &Column, proposed: &Column) -> Column {
    if current.find.nav <= proposed.find.nav {
        return proposed.clone();
    }
    let mut next = proposed.clone();
    next.find.nav = current.find.nav;
    next.find.pending = false;
    next
}

fn owe_landing(column: &mut Column, here: &Place) {
    column.find.pending = true;
    column.find.origin = here.clone();
    let epoch = nav_epoch();
    if epoch > column.find.nav {
        column.find.nav = epoch;
    }
}

/// Drop every find fact that depends on the character contract.
/// The query and the find mode stay. The owed landing does not.
/// A closed find does not start a scan.
pub fn invalidate_find_text(column: &Column) -> Column {
    let mut next = column.clone();
    let query = next.find.query.clone();
    let open = next.mode == Mode::Find && !query.trim().is_empty();
    begin_query(&mut next, &query);
    next.find.pending = false;
    if !open {
        next.find.phase = Phase::Idle;
    }
    next
}

/// The shell applies one published snapshot here. The column owns the owed
/// landing. A snapshot epoch newer than the search cancels it before the
/// place is consulted, so a same-chapter page or a gesture still at the
/// origin cannot land. Leaving the origin remains the backstop for a move
/// that changed the href, the offset, or the PDF page.
pub fn on_scan_publish(column: &Column, scan: &Scan, live: &Place) -> (Column, Option<Hit>) {
    let mut next = column.clone();
    if scan.nav > next.find.nav {
        next.find.pending = false;
        next.find.nav = scan.nav;
    }
    if next.find.pending && reader_left(&next.find.origin, live) {
        next.find.pending = false;
    }
    let anchor = if next.find.pending {
        next.find.origin.clone()
    } else {
        live.clone()
    };
    replace_scan(&next, scan, &anchor)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scan {
    pub book: String,
    pub query: String,
    pub generation: u64,
    pub hits: Vec<Hit>,
    pub seen: u32,
    pub total: u32,
    pub text_units: u32,
    pub empty_units: u32,
    pub failed_units: u32,
    pub done: bool,
    /// Navigation epoch at the moment the stage published this snapshot.
    pub nav: u64,
}

/// The stage sends the hits it has so far. An older generation or another
/// book does not replace the list. The first snapshot can take one pending step.
pub fn replace_scan(column: &Column, scan: &Scan, here: &Place) -> (Column, Option<Hit>) {
    if scan.book != column.book
        || scan.query != column.find.query
        || scan.generation != column.find.generation
    {
        return (column.clone(), None);
    }
    let mut next = column.clone();
    next.find.hits = scan.hits.clone();
    next.find.seen = scan.seen;
    next.find.total = scan.total;
    next.find.text_units = scan.text_units;
    next.find.empty_units = scan.empty_units;
    next.find.failed_units = scan.failed_units;
    next.find.phase = if !scan.done {
        Phase::Searching
    } else if scan.failed_units > 0 {
        Phase::Incomplete
    } else {
        Phase::Done
    };
    if next
        .find
        .index
        .is_some_and(|index| index >= next.find.hits.len())
    {
        next.find.index = None;
    }
    let jump = if next.find.pending {
        pending_hit(&next.order, &next.find.hits, here, scan.done).map(|index| {
            next.find.pending = false;
            next.find.index = Some(index);
            let hit = next.find.hits[index].clone();
            if !same_anchor(&hit, here) {
                next.find.navigated = true;
            }
            hit
        })
    } else {
        None
    };
    (next, jump)
}

/// Find-next, find-previous, and Shift+Return are registered only while
/// find is open and the field is not composing. ⌘F stays a window binding.
pub fn find_navigation_mounted(mode: Mode, composing: bool) -> bool {
    mode == Mode::Find && !composing
}

pub fn choose(column: &Column, index: usize, here: &Place) -> (Column, Option<Hit>) {
    if column.find.composing || index >= column.find.hits.len() {
        return (column.clone(), None);
    }
    let mut next = column.clone();
    next.find.pending = false;
    next.find.index = Some(index);
    let hit = next.find.hits[index].clone();
    if !same_anchor(&hit, here) {
        next.find.navigated = true;
    }
    (next, Some(hit))
}

pub fn find_line(column: &Column) -> FindLine {
    let find = &column.find;
    if find.query.trim().is_empty() || find.phase == Phase::Idle && find.hits.is_empty() {
        return FindLine {
            text: String::new(),
            retry: false,
        };
    }
    if find.failed_units > 0 || find.phase == Phase::Incomplete {
        let extra = if find.hits.is_empty() {
            String::new()
        } else {
            format!(" · {} 个结果", find.hits.len())
        };
        return FindLine {
            text: format!("查找未完成，可重试{extra}"),
            retry: true,
        };
    }
    if find.phase == Phase::Searching {
        return FindLine {
            text: format!("搜索中 · 已搜 {}/{}", find.seen, find.total),
            retry: false,
        };
    }
    if find.hits.is_empty() && find.text_units == 0 {
        return FindLine {
            text: "没有可搜索的文本".to_string(),
            retry: false,
        };
    }
    if find.hits.is_empty() {
        return FindLine {
            text: "无结果 · 已搜全书可用文本".to_string(),
            retry: false,
        };
    }
    FindLine {
        text: format!(
            "{} 个结果 · 已搜 {}/{}",
            find.hits.len(),
            find.seen,
            find.total
        ),
        retry: false,
    }
}

pub fn hits_in(text: &str, query: &str, page: u32, href: &str, kind: IndexKind) -> Vec<Hit> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find(query) {
        let byte = from + at;
        let prefix = &text[..byte];
        let start_chars = prefix.chars().count() as u32;
        let start_utf16 = prefix.encode_utf16().count() as u32;
        let (start, end) = match kind {
            IndexKind::Chars => (start_chars, start_chars + query.chars().count() as u32),
            IndexKind::Utf16 => (
                start_utf16,
                start_utf16 + query.encode_utf16().count() as u32,
            ),
        };
        hits.push(Hit {
            page,
            href: href.to_string(),
            start,
            end,
            kind,
            snippet: snippet(text, byte, query.len()),
        });
        let next = byte + query.len();
        if next <= from {
            break;
        }
        from = next;
    }
    hits
}

fn snippet(text: &str, byte: usize, query_len: usize) -> String {
    let start = text[..byte].chars().rev().take(12).count();
    let prefix_chars = text[..byte].chars().count();
    let from = prefix_chars - start;
    let query_chars = text[byte..byte + query_len].chars().count();
    let take = from + query_chars + 12;
    let mut out: String = text.chars().skip(from).take(take).collect();
    out = out.replace(['\n', '\r', '\t'], " ");
    out
}

/// The first bookmark or search jump fills an empty return slot.
/// A later jump keeps that place. The same place does not record one.
pub fn remember_origin(existing: Option<Place>, here: &Place, dest: &Place) -> Option<Place> {
    if existing.is_some() {
        return existing;
    }
    if here == dest {
        return None;
    }
    Some(here.clone())
}

pub fn same_place(mark: &Bookmark, page: u32, href: &str, offset: u32) -> bool {
    if href.is_empty() && mark.href.is_empty() {
        mark.page == page
    } else {
        spike_epub::normalize_href(&mark.href) == spike_epub::normalize_href(href)
            && mark.offset == offset
    }
}

/// The row the add button stores. A nearby passage is the label. Without
/// one, a PDF names its page and an EPUB names the chapter file and offset.
pub fn bookmark_label(place: &Place, nearby: &str) -> String {
    let flat = nearby.split_whitespace().collect::<Vec<_>>().join(" ");
    let excerpt: String = flat.chars().take(32).collect();
    if !excerpt.is_empty() {
        return excerpt;
    }
    if place.href.is_empty() {
        return format!("第{}页", place.page.max(1));
    }
    let name = place.href.split('#').next().unwrap_or(place.href.as_str());
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    if name.is_empty() {
        format!("第{}字", place.offset)
    } else {
        format!("{name} · 第{}字", place.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::{begin_card, CardKey, MarginInk, NoteField};

    fn live(book: &str, page: u32) -> LivePage {
        LivePage {
            book: book.to_string(),
            spine: "c1.xhtml".to_string(),
            page,
            page_w: 100.0,
            page_h: 100.0,
            moving: false,
        }
    }

    fn here(page: u32) -> Place {
        Place {
            href: "c1.xhtml".to_string(),
            page,
            offset: 0,
        }
    }

    fn pdf_page(page: u32) -> Place {
        Place {
            href: String::new(),
            page,
            offset: 0,
        }
    }

    fn publish(
        book: &str,
        query: &str,
        generation: u64,
        hits: Vec<Hit>,
        seen: u32,
        total: u32,
        done: bool,
    ) -> Scan {
        Scan {
            book: book.to_string(),
            query: query.to_string(),
            generation,
            hits,
            seen,
            total,
            text_units: seen,
            empty_units: 0,
            failed_units: 0,
            done,
            nav: 0,
        }
    }

    #[test]
    fn a_bookmark_survives_reopen_undo_and_stays_on_its_book() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path();
        let marks = Marks::open(path).unwrap();
        let mut column = Column {
            book: "a.epub".to_string(),
            ..Column::default()
        };
        column = toggle_mark(
            &column,
            &marks,
            &Place {
                href: "c1.xhtml".to_string(),
                page: 2,
                offset: 4,
            },
            "信",
            0,
        )
        .unwrap();
        assert_eq!(column.marks.len(), 1);
        assert_eq!(column.marks[0].label, "信");
        drop(marks);
        let marks = Marks::open(path).unwrap();
        let loaded = marks.list("a.epub").unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(marks.list("b.epub").unwrap().is_empty());
        let id = loaded[0].id;
        column.marks = loaded;
        column = delete_mark(&column, &marks, id, 1_000).unwrap();
        assert!(column.marks.is_empty());
        assert!(column.undo.is_some());
        column = undo_delete(&column, &marks).unwrap();
        assert_eq!(column.marks.len(), 1);
        let again = Marks::open(path).unwrap();
        assert_eq!(again.list("a.epub").unwrap().len(), 1);
        assert!(again.list("b.epub").unwrap().is_empty());
        let held = Column {
            undo: column.marks.first().cloned(),
            undo_until: 5_000,
            ..column.clone()
        };
        assert!(expire_undo(&held, 4_999).undo.is_some());
        assert!(expire_undo(&held, 5_000).undo.is_none());
        let other = clear_book(&column, "b.epub");
        assert!(other.marks.is_empty());
        assert_eq!(other.book, "b.epub");
        assert_eq!(clear_book(&other, "b.epub").book, "b.epub");
    }

    #[test]
    fn a_query_crosses_units_and_a_late_or_ime_result_does_not_land() {
        let text = "天亮以后。才想起今天要把信送到镇上。";
        let first = hits_in(text, "信", 1, "c1.xhtml", IndexKind::Utf16);
        assert_eq!(first.len(), 1);
        let prefix = text.split('信').next().unwrap();
        assert_eq!(first[0].start, prefix.encode_utf16().count() as u32);
        let more = hits_in("河边还有信。", "信", 2, "c2.xhtml", IndexKind::Utf16);
        let column = type_find(
            &Column {
                book: "a.epub".to_string(),
                mode: Mode::Find,
                ..Column::default()
            },
            "信",
        );
        assert!(column.find.hits.is_empty());
        assert_eq!(column.find.phase, Phase::Searching);
        let report = Scan {
            book: "a.epub".to_string(),
            query: "信".to_string(),
            generation: column.find.generation,
            hits: first.clone(),
            seen: 1,
            total: 2,
            text_units: 1,
            empty_units: 0,
            failed_units: 0,
            done: false,
            nav: 0,
        };
        let (column, jump) = on_scan_publish(&column, &report, &here(1));
        assert!(jump.is_none());
        assert_eq!(column.find.hits.len(), 1);
        let stale = Scan {
            generation: column.find.generation - 1,
            hits: more.clone(),
            ..report.clone()
        };
        let (held, _) = on_scan_publish(&column, &stale, &here(1));
        assert_eq!(held.find.hits.len(), 1);
        let other_book = Scan {
            book: "b.epub".to_string(),
            hits: more.clone(),
            ..report.clone()
        };
        let (held, _) = on_scan_publish(&column, &other_book, &here(1));
        assert_eq!(held.find.hits.len(), 1);
        let typed = type_find(&column, "袋");
        assert!(typed.find.hits.is_empty());
        assert_ne!(typed.find.generation, column.find.generation);
        let late = Scan {
            query: "信".to_string(),
            generation: column.find.generation,
            hits: first,
            ..report
        };
        let (still, _) = on_scan_publish(&typed, &late, &here(1));
        assert!(still.find.hits.is_empty());
        let mut marked = column.clone();
        marked.find.hits = more.clone();
        marked.find.phase = Phase::Done;
        marked.find.seen = 2;
        marked.find.total = 2;
        marked.find.text_units = 2;
        let composing = compose_find(&marked, true);
        let drafted = type_find(&composing, "袋");
        assert_eq!(drafted.find.query, "信");
        assert_eq!(drafted.find.hits.len(), 1);
        let (returned, jump) = find_return(&drafted, &here(1));
        assert!(jump.is_none());
        assert_eq!(returned.find.query, "信");
        let (stepped, jump) = find_return(&compose_find(&returned, false), &here(1));
        assert!(jump.is_none());
        assert_eq!(stepped.find.query, "袋");
        assert!(stepped.find.pending);
        assert!(stepped.find.hits.is_empty());
        let scan = Scan {
            book: "a.epub".to_string(),
            query: "袋".to_string(),
            generation: stepped.find.generation,
            hits: hits_in("我把信放进袋", "袋", 4, "c2.xhtml", IndexKind::Utf16),
            seen: 2,
            total: 2,
            text_units: 2,
            empty_units: 0,
            failed_units: 0,
            done: true,
            nav: 0,
        };
        let late = Scan {
            query: "信".to_string(),
            generation: column.find.generation,
            ..scan.clone()
        };
        let (held, jump) = on_scan_publish(&stepped, &late, &here(1));
        assert!(jump.is_none());
        assert!(held.find.hits.is_empty());
        let (landed, jump) = on_scan_publish(&stepped, &scan, &here(1));
        assert!(jump.is_some());
        assert_eq!(landed.find.hits.len(), 1);
        assert!(landed.find.navigated);
        let (again, jump) = on_scan_publish(&landed, &scan, &here(4));
        assert!(jump.is_none());
        assert_eq!(again.find.hits.len(), 1);
    }

    #[test]
    fn empty_failed_and_unfinished_are_not_zero_results() {
        let column = type_find(
            &Column {
                book: "a.pdf".to_string(),
                ..Column::default()
            },
            "信",
        );
        let base = Scan {
            book: "a.pdf".to_string(),
            query: "信".to_string(),
            generation: column.find.generation,
            hits: Vec::new(),
            seen: 1,
            total: 2,
            text_units: 0,
            empty_units: 1,
            failed_units: 0,
            done: false,
            nav: 0,
        };
        let (column, _) = on_scan_publish(&column, &base, &pdf_page(1));
        assert_eq!(find_line(&column).text, "搜索中 · 已搜 1/2");
        let failed = Scan {
            seen: 2,
            empty_units: 1,
            failed_units: 1,
            done: true,
            ..base.clone()
        };
        let (column, _) = on_scan_publish(&column, &failed, &pdf_page(1));
        let line = find_line(&column);
        assert!(line.retry);
        assert!(line.text.starts_with("查找未完成，可重试"));
        assert!(!line.text.contains("无结果"));
        let mut only_empty = type_find(&Column::default(), "信");
        only_empty.book = "a.pdf".to_string();
        let done = Scan {
            seen: 1,
            total: 1,
            empty_units: 1,
            failed_units: 0,
            done: true,
            generation: only_empty.find.generation,
            ..base
        };
        let (only_empty, _) = on_scan_publish(&only_empty, &done, &pdf_page(1));
        assert_eq!(find_line(&only_empty).text, "没有可搜索的文本");
        let none = type_find(
            &Column {
                book: "a.pdf".to_string(),
                ..Column::default()
            },
            "缺",
        );
        let scanned = Scan {
            book: "a.pdf".to_string(),
            query: "缺".to_string(),
            generation: none.find.generation,
            hits: Vec::new(),
            seen: 1,
            total: 1,
            text_units: 1,
            empty_units: 0,
            failed_units: 0,
            done: true,
            nav: 0,
        };
        let (none, _) = on_scan_publish(&none, &scanned, &pdf_page(1));
        assert_eq!(find_line(&none).text, "无结果 · 已搜全书可用文本");
        let retry = retry_find(&none);
        assert!(retry.find.hits.is_empty());
        assert_eq!(retry.find.phase, Phase::Searching);
        assert_eq!(retry.find.query, "缺");
        assert_ne!(retry.find.generation, none.find.generation);
    }

    #[test]
    fn escape_restores_the_same_editor_and_a_jump_does_not() {
        let ink = MarginInk {
            new_note: NoteField {
                body: "草稿".to_string(),
                composing: false,
                passage: None,
                fingerprint: None,
                notice: String::new(),
            },
            place_book: "a.epub".to_string(),
            place_page: 3,
            place_spine: "c1.xhtml".to_string(),
            ..MarginInk::default()
        };
        let page = live("a.epub", 3);
        let column = Column {
            book: "a.epub".to_string(),
            mode: Mode::Notes,
            ..Column::default()
        };
        let (open, released) = enter_find(&column, &ink, &page);
        assert_eq!(open.mode, Mode::Find);
        assert!(!released.new_note.composing);
        assert_eq!(released.new_note.body, "草稿");
        assert!(open.find.seat.is_some());
        let closed = close_find(&open);
        assert_eq!(closed.column.mode, Mode::Notes);
        let focused = refocus_after_find(&released, closed.seat.as_ref(), &page);
        assert!(focused.focus_new > released.focus_new);
        assert_eq!(focused.new_note.body, "草稿");
        let mut moved = open.clone();
        moved.find.navigated = true;
        let closed = close_find(&moved);
        assert!(closed.seat.is_none());
        let left = refocus_after_find(&released, closed.seat.as_ref(), &page);
        assert_eq!(left.focus_new, released.focus_new);
        let elsewhere = live("a.epub", 4);
        let closed = close_find(&open);
        let skipped = refocus_after_find(&released, closed.seat.as_ref(), &elsewhere);
        assert_eq!(skipped.focus_new, released.focus_new);
        let key = CardKey {
            book: "a.epub".to_string(),
            note_id: 7,
            kind: CardKind::Edit,
        };
        let card = begin_card(&MarginInk::default(), key, "卡内");
        let (open, parked) = enter_marks(&column, &card);
        assert_eq!(open.mode, Mode::Marks);
        assert!(matches!(parked.face, Face::New));
        assert_eq!(parked.drafts[0].body, "卡内");
        let (open, held) = enter_find(&column, &card, &page);
        let closed = close_find(&open);
        let back = restore_after_find(&closed.column, &held, closed.seat.as_ref(), &page);
        assert!(matches!(back.face, Face::Card(_)));
        let mut jumped = open.clone();
        jumped.find.navigated = true;
        let closed = close_find(&jumped);
        let parked = restore_after_find(&closed.column, &held, closed.seat.as_ref(), &page);
        assert!(matches!(parked.face, Face::New));
        assert_eq!(parked.drafts[0].body, "卡内");
        let closed = close_find(&open);
        let other = live("a.epub", 9);
        let parked = restore_after_find(&closed.column, &held, closed.seat.as_ref(), &other);
        assert!(matches!(parked.face, Face::New));
        assert_eq!(parked.drafts[0].body, "卡内");
    }

    #[test]
    fn the_first_jump_records_one_origin() {
        let start = here(2);
        let dest = Place {
            href: "c2.xhtml".to_string(),
            page: 5,
            offset: 8,
        };
        let once = remember_origin(None, &start, &dest);
        assert_eq!(once.as_ref(), Some(&start));
        let again = remember_origin(once.clone(), &dest, &here(9));
        assert_eq!(again, once);
        assert_eq!(remember_origin(None, &start, &start), None);
        let mut column = type_find(
            &Column {
                book: "a.epub".to_string(),
                mode: Mode::Find,
                ..Column::default()
            },
            "信",
        );
        column.find.hits = hits_in("信送到镇上", "信", 5, "c2.xhtml", IndexKind::Utf16);
        column.find.phase = Phase::Done;
        column.find.seen = 1;
        column.find.total = 1;
        column.find.text_units = 1;
        let (next, hit) = find_return(&column, &start);
        assert!(hit.is_some());
        assert!(next.find.navigated);
        let (later, _) = step(&next, true, &dest);
        assert!(later.find.navigated);
        let (returned, jump) = find_return(&compose_find(&column, true), &start);
        assert!(jump.is_none());
        assert!(!returned.find.navigated);
    }

    fn spine() -> Vec<String> {
        (1..=5).map(|n| format!("c{n}.xhtml")).collect()
    }

    fn epub_hits() -> Vec<Hit> {
        let mut hits = hits_in("信在第一章", "信", 1, "c1.xhtml", IndexKind::Utf16);
        hits.extend(hits_in(
            "第五章信甲信乙",
            "信",
            5,
            "c5.xhtml",
            IndexKind::Utf16,
        ));
        hits
    }

    #[test]
    fn epub_next_and_previous_follow_the_spine() {
        let hits = epub_hits();
        assert_eq!(hits[0].page, 1);
        assert_eq!(hits[1].page, 5);
        let column = Column {
            book: "a.epub".to_string(),
            mode: Mode::Find,
            order: spine(),
            find: Find {
                query: "信".to_string(),
                draft: "信".to_string(),
                hits: hits.clone(),
                phase: Phase::Done,
                seen: 5,
                total: 5,
                text_units: 2,
                ..Find::default()
            },
            ..Column::default()
        };
        let chapter5 = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let (forward, hit) = step(&column, true, &chapter5);
        let hit = hit.expect("next");
        assert_eq!(hit.href, "c5.xhtml");
        assert_eq!(hit.start, hits[1].start);
        assert_ne!(hit.href, "c1.xhtml");
        let covered = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: hits[1].start,
        };
        let (again, hit) = find_return(&column, &covered);
        assert_eq!(hit.unwrap().start, hits[1].start);
        assert!(!again.find.navigated);
        let adjacent = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: hits[1].start + 1,
        };
        let (_, hit) = step(&column, true, &adjacent);
        assert_eq!(hit.unwrap().start, hits[2].start);
        let (_, back) = step(&forward, false, &chapter5);
        assert_eq!(back.unwrap().href, "c1.xhtml");
        let (wrapped, _) = step(&forward, true, &chapter5);
        let (wrapped, hit) = step(&wrapped, true, &chapter5);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert_eq!(wrapped.find.index, Some(0));
        let (from_start, hit) = step(&wrapped, false, &chapter5);
        assert_eq!(hit.unwrap().start, hits[2].start);
        assert_eq!(from_start.find.index, Some(2));
        assert!(pending_hit(&spine(), &hits[..1], &chapter5, false).is_none());
        assert_eq!(pending_hit(&spine(), &hits, &chapter5, false), Some(1));
        assert_eq!(pending_hit(&spine(), &hits[..1], &chapter5, true), Some(0));
        let pdf_here = Place {
            href: String::new(),
            page: 3,
            offset: 0,
        };
        let pdf_hits = vec![
            hits_in("信", "信", 1, "", IndexKind::Chars)[0].clone(),
            hits_in("信", "信", 3, "", IndexKind::Chars)[0].clone(),
        ];
        let pdf = Column {
            book: "a.pdf".to_string(),
            mode: Mode::Find,
            find: Find {
                query: "信".to_string(),
                draft: "信".to_string(),
                hits: pdf_hits,
                phase: Phase::Done,
                ..Find::default()
            },
            ..Column::default()
        };
        let (_, hit) = step(&pdf, true, &pdf_here);
        assert_eq!(hit.unwrap().page, 3);
    }

    /// The stage publishes one chapter at a time. The column, the pending
    /// jump, and the next step have to use that same coverage. A hit in an
    /// earlier chapter does not select, navigate, or wrap while the scan
    /// has not reached the reader.
    #[test]
    fn a_partial_publish_does_not_select_before_the_reader() {
        let here = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let seat = EditorSeat {
            book: "a.epub".to_string(),
            page: 1,
            href: "c5.xhtml".to_string(),
            kind: SeatKind::New,
        };
        let column = Column {
            book: "a.epub".to_string(),
            mode: Mode::Find,
            order: spine(),
            find: Find {
                query: "信".to_string(),
                draft: "信".to_string(),
                pending: true,
                origin: here.clone(),
                phase: Phase::Searching,
                seat: Some(seat),
                ..Find::default()
            },
            ..Column::default()
        };
        let early = epub_hits().into_iter().take(1).collect::<Vec<_>>();
        let partial = Scan {
            book: "a.epub".to_string(),
            query: "信".to_string(),
            generation: 0,
            hits: early.clone(),
            seen: 1,
            total: 5,
            text_units: 1,
            empty_units: 0,
            failed_units: 0,
            done: false,
            nav: 0,
        };
        let (column, hit) = on_scan_publish(&column, &partial, &here);
        assert!(hit.is_none());
        assert!(column.find.index.is_none());
        assert!(!column.find.navigated);
        assert!(column.find.pending);
        assert!(pending_hit(&spine(), &early, &here, false).is_none());
        let (stepped, hit) = step(&column, true, &here);
        assert!(hit.is_none());
        assert!(stepped.find.index.is_none());
        assert!(!stepped.find.navigated);
        let closed = close_find(&stepped);
        assert!(closed.seat.is_some());

        let (back, hit) = step(&column, false, &here);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert!(back.find.navigated);
        let (back_again, hit) = step(&back, false, &here);
        assert!(hit.is_none());
        assert_eq!(back_again.find.index, Some(0));

        let all = epub_hits();
        let crossing = Scan {
            book: "a.epub".to_string(),
            query: "信".to_string(),
            generation: 0,
            hits: all.clone(),
            seen: 4,
            total: 5,
            text_units: 2,
            empty_units: 2,
            failed_units: 0,
            done: false,
            nav: 0,
        };
        let (column, hit) = on_scan_publish(&column, &crossing, &here);
        let hit = hit.expect("chapter 5 is on the page");
        assert_eq!(hit.href, "c5.xhtml");
        assert_eq!(column.find.index, pending_hit(&spine(), &all, &here, false));
        assert_ne!(column.find.index, Some(0));
        assert!(column.find.navigated);
        assert!(!column.find.pending);
        let (next, hit) = step(&column, true, &here);
        assert_eq!(hit.unwrap().href, "c5.xhtml");
        assert_eq!(next.find.index, Some(column.find.index.unwrap() + 1));
        let closed = close_find(&column);
        assert!(closed.seat.is_none());

        let finished_early = Scan {
            hits: early.clone(),
            seen: 5,
            total: 5,
            done: true,
            ..partial
        };
        let fresh = Column {
            find: Find {
                pending: true,
                origin: here.clone(),
                phase: Phase::Searching,
                seat: Some(EditorSeat {
                    book: "a.epub".to_string(),
                    page: 1,
                    href: "c5.xhtml".to_string(),
                    kind: SeatKind::New,
                }),
                query: "信".to_string(),
                draft: "信".to_string(),
                ..Find::default()
            },
            ..column.clone()
        };
        let (done, hit) = on_scan_publish(&fresh, &finished_early, &here);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert_eq!(done.find.index, Some(0));
        assert!(done.find.navigated);
        assert_eq!(pending_hit(&spine(), &early, &here, true), Some(0));
        assert!(close_find(&done).seat.is_none());
    }

    #[test]
    fn a_manual_move_cancels_the_owed_landing() {
        let chapter5 = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let seat = EditorSeat {
            book: "a.epub".to_string(),
            page: 1,
            href: "c5.xhtml".to_string(),
            kind: SeatKind::New,
        };
        let opened = type_find(
            &Column {
                book: "a.epub".to_string(),
                mode: Mode::Find,
                order: spine(),
                find: Find {
                    seat: Some(seat.clone()),
                    ..Find::default()
                },
                ..Column::default()
            },
            "信",
        );
        let (column, jump) = find_return(&opened, &chapter5);
        assert!(jump.is_none());
        assert!(column.find.pending);
        assert_eq!(column.find.origin.href, "c5.xhtml");
        let early = epub_hits().into_iter().take(1).collect::<Vec<_>>();
        let partial = publish("a.epub", "信", column.find.generation, early, 1, 5, false);
        let (waiting, jump) = on_scan_publish(&column, &partial, &chapter5);
        assert!(jump.is_none());
        assert!(waiting.find.pending);
        assert!(waiting.find.index.is_none());

        let (back, hit) = step(&waiting, false, &chapter5);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert!(!back.find.pending);
        assert_eq!(back.find.index, Some(0));
        assert!(back.find.navigated);
        let later = publish(
            "a.epub",
            "信",
            back.find.generation,
            epub_hits(),
            5,
            5,
            true,
        );
        let (held, jump) = on_scan_publish(&back, &later, &chapter5);
        assert!(jump.is_none());
        assert_eq!(held.find.index, Some(0));
        assert_eq!(held.find.hits[0].href, "c1.xhtml");
        assert!(close_find(&held).seat.is_none());
        let (forward, hit) = step(&held, true, &chapter5);
        assert_eq!(hit.unwrap().href, "c5.xhtml");
        assert_eq!(forward.find.index, Some(1));

        let (clicked, hit) = choose(&waiting, 0, &chapter5);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert!(!clicked.find.pending);
        assert_eq!(clicked.find.index, Some(0));
        assert!(clicked.find.navigated);
        let (still, jump) = on_scan_publish(&clicked, &later, &chapter5);
        assert!(jump.is_none());
        assert_eq!(still.find.index, Some(0));

        let same = choose(&waiting, 0, &hit_place(&waiting.find.hits[0]));
        let (same, _) = same;
        assert!(!same.find.pending);
        assert!(!same.find.navigated);
        assert!(close_find(&same).seat.is_some());
        let (quiet, jump) = on_scan_publish(&same, &later, &hit_place(&waiting.find.hits[0]));
        assert!(jump.is_none());
        assert_eq!(quiet.find.index, Some(0));

        let turned = release_landing(&waiting);
        assert!(!turned.find.pending);
        let page6 = Place {
            href: "c6.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let (stayed, jump) = on_scan_publish(&waiting, &later, &page6);
        assert!(jump.is_none());
        assert!(!stayed.find.pending);
        assert!(stayed.find.index.is_none());
        let (after_key, hit) = step(&stayed, true, &page6);
        assert_eq!(hit.unwrap().href, "c1.xhtml");
        assert_eq!(after_key.find.index, Some(0));

        let reflowed = Place {
            href: "c5.xhtml".to_string(),
            page: 4,
            offset: 0,
        };
        let (still_owed, jump) = on_scan_publish(&waiting, &partial, &reflowed);
        assert!(jump.is_none());
        assert!(still_owed.find.pending);

        let pdf = type_find(
            &Column {
                book: "a.pdf".to_string(),
                mode: Mode::Find,
                ..Column::default()
            },
            "信",
        );
        let page5 = pdf_page(5);
        let (pdf, _) = find_return(&pdf, &page5);
        let only_first = publish(
            "a.pdf",
            "信",
            pdf.find.generation,
            hits_in("信", "信", 1, "", IndexKind::Utf16),
            1,
            5,
            false,
        );
        let (pdf_waiting, jump) = on_scan_publish(&pdf, &only_first, &page5);
        assert!(jump.is_none());
        assert!(pdf_waiting.find.pending);
        assert!(pdf_waiting.find.index.is_none());
        let (pdf_back, hit) = step(&pdf_waiting, false, &page5);
        assert_eq!(hit.unwrap().page, 1);
        assert!(!pdf_back.find.pending);
        let pdf_done = publish(
            "a.pdf",
            "信",
            pdf_back.find.generation,
            {
                let mut hits = hits_in("信", "信", 1, "", IndexKind::Utf16);
                hits.extend(hits_in("信", "信", 5, "", IndexKind::Utf16));
                hits
            },
            5,
            5,
            true,
        );
        let (pdf_held, jump) = on_scan_publish(&pdf_back, &pdf_done, &page5);
        assert!(jump.is_none());
        assert_eq!(pdf_held.find.index, Some(0));
        assert_eq!(pdf_held.find.hits[0].page, 1);
        let (pdf_click, _) = choose(&pdf_waiting, 0, &page5);
        assert!(!pdf_click.find.pending);
        let (pdf_click, jump) = on_scan_publish(&pdf_click, &pdf_done, &page5);
        assert!(jump.is_none());
        assert_eq!(pdf_click.find.index, Some(0));
        let paged = release_landing(&pdf_waiting);
        let (paged, jump) = on_scan_publish(&paged, &pdf_done, &pdf_page(6));
        assert!(jump.is_none());
        assert!(paged.find.index.is_none());
    }

    #[test]
    fn a_navigation_epoch_cancels_before_the_place_moves() {
        let chapter5 = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let page_only = Place {
            href: "c5.xhtml".to_string(),
            page: 2,
            offset: 0,
        };
        let opened = type_find(
            &Column {
                book: "a.epub".to_string(),
                mode: Mode::Find,
                order: spine(),
                ..Column::default()
            },
            "信",
        );
        let (waiting, jump) = find_return(&opened, &chapter5);
        assert!(jump.is_none());
        assert!(waiting.find.pending);
        let hit = Hit {
            page: 5,
            href: "c5.xhtml".to_string(),
            start: 10,
            end: 11,
            kind: IndexKind::Utf16,
            snippet: "信".to_string(),
        };
        let mut owed = publish(
            "a.epub",
            "信",
            waiting.find.generation,
            vec![hit.clone()],
            5,
            5,
            true,
        );
        owed.nav = waiting.find.nav;
        let zero = page_command(0, Some("edge"));
        assert_eq!(zero.delta, 0);
        assert!(zero.intent.is_none());
        let (still, jump) = on_scan_publish(&waiting, &owed, &page_only);
        let jump = jump.expect("a column-page change is still the origin");
        assert_eq!(jump.href, "c5.xhtml");
        assert_eq!(jump.start, 10);
        assert!(!still.find.pending);

        let edge = page_command(1, Some("edge"));
        let key = page_command(-1, Some("key"));
        let legacy = page_command(1, None);
        assert_eq!(edge.intent, Some(NavIntent::PageEdge));
        assert_eq!(key.intent, Some(NavIntent::Key));
        assert_eq!(legacy.intent, Some(NavIntent::Key));

        let intents = [
            edge.intent.unwrap(),
            key.intent.unwrap(),
            NavIntent::Trackpad,
            NavIntent::PdfGlide,
            NavIntent::Toc,
            NavIntent::Link,
            NavIntent::Bookmark,
        ];
        for intent in intents {
            let epoch = arm_nav(intent);
            assert_eq!(last_intent_tag(), intent_tag(intent));
            assert!(epoch > waiting.find.nav);
            let mut scan = owed.clone();
            scan.nav = epoch;
            let (held, jump) = on_scan_publish(&waiting, &scan, &page_only);
            assert!(jump.is_none(), "{intent:?} page-only");
            assert!(!held.find.pending, "{intent:?}");
            assert!(held.find.index.is_none(), "{intent:?}");
            assert_eq!(held.find.nav, epoch);
            let (same, jump) = on_scan_publish(&waiting, &scan, &chapter5);
            assert!(jump.is_none(), "{intent:?} same place");
            assert!(!same.find.pending, "{intent:?}");
        }

        let pdf = type_find(
            &Column {
                book: "a.pdf".to_string(),
                mode: Mode::Find,
                ..Column::default()
            },
            "信",
        );
        let page5 = pdf_page(5);
        let (pdf, _) = find_return(&pdf, &page5);
        let mut glide = publish(
            "a.pdf",
            "信",
            pdf.find.generation,
            hits_in("信", "信", 5, "", IndexKind::Utf16),
            5,
            5,
            true,
        );
        glide.nav = pdf.find.nav;
        let (landed, jump) = on_scan_publish(&pdf, &glide, &page5);
        assert!(jump.is_some());
        assert_eq!(landed.find.hits[0].page, 5);
        let epoch = arm_nav(NavIntent::PdfGlide);
        glide.nav = epoch;
        let (held, jump) = on_scan_publish(&pdf, &glide, &page5);
        assert!(jump.is_none());
        assert!(!held.find.pending);
        assert!(held.find.index.is_none());

        let proposed = waiting.clone();
        let mut current = waiting.clone();
        current.find.nav = arm_nav(NavIntent::Key);
        current.find.pending = false;
        let settled = observe_nav(&current, &proposed);
        assert_eq!(settled.find.nav, current.find.nav);
        assert!(!settled.find.pending);
        assert_eq!(settled.find.query, proposed.find.query);
    }

    #[test]
    fn a_text_contract_change_drops_the_old_offsets() {
        let origin = Place {
            href: "c1.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        let old = Hit {
            page: 1,
            href: "c1.xhtml".to_string(),
            start: 2,
            end: 3,
            kind: IndexKind::Utf16,
            snippet: "—".to_string(),
        };
        let column = Column {
            book: "a.epub".to_string(),
            mode: Mode::Find,
            order: spine(),
            find: Find {
                draft: "—".to_string(),
                query: "—".to_string(),
                generation: 4,
                hits: vec![old.clone()],
                index: Some(0),
                pending: true,
                origin: origin.clone(),
                phase: Phase::Searching,
                ..Find::default()
            },
            ..Column::default()
        };
        let cleared = invalidate_find_text(&column);
        assert_eq!(cleared.mode, Mode::Find);
        assert_eq!(cleared.find.query, "—");
        assert_eq!(cleared.find.draft, "—");
        assert!(cleared.find.hits.is_empty());
        assert!(cleared.find.index.is_none());
        assert!(!cleared.find.pending);
        assert!(cleared.find.generation > column.find.generation);
        assert_eq!(cleared.find.phase, Phase::Searching);
        let (stepped, hit) = step(&cleared, true, &origin);
        assert!(hit.is_none());
        assert!(stepped.find.hits.is_empty());
        let (chosen, hit) = choose(&cleared, 0, &origin);
        assert!(hit.is_none());
        assert!(chosen.find.index.is_none());

        let stale = publish("a.epub", "—", column.find.generation, vec![old], 1, 1, true);
        let (held, jump) = on_scan_publish(&cleared, &stale, &origin);
        assert!(jump.is_none());
        assert!(held.find.hits.is_empty());
        assert!(!held.find.pending);

        let fresh_hit = Hit {
            page: 1,
            href: "c1.xhtml".to_string(),
            start: 1,
            end: 2,
            kind: IndexKind::Utf16,
            snippet: "—".to_string(),
        };
        let mut fresh = publish(
            "a.epub",
            "—",
            cleared.find.generation,
            vec![fresh_hit],
            1,
            1,
            true,
        );
        fresh.nav = cleared.find.nav;
        let (rescanned, jump) = on_scan_publish(&held, &fresh, &origin);
        assert!(jump.is_none());
        assert!(!rescanned.find.pending);
        assert_eq!(rescanned.find.hits[0].start, 1);
        let (moved, hit) = step(&rescanned, true, &origin);
        assert_eq!(hit.unwrap().start, 1);
        assert_eq!(moved.find.index, Some(0));

        let mut closed = column.clone();
        closed.mode = Mode::Notes;
        let idle = invalidate_find_text(&closed);
        assert_eq!(idle.mode, Mode::Notes);
        assert_eq!(idle.find.query, "—");
        assert!(idle.find.hits.is_empty());
        assert_eq!(idle.find.phase, Phase::Idle);
        assert!(!idle.find.pending);
    }

    #[test]
    fn navigation_and_bold_share_one_gate() {
        let stage = include_str!("../../shell/src/stage.rs");
        let web = include_str!("../../shell/src/web.rs");
        let shell = include_str!("../../shell/src/lib.rs");
        let form = include_str!("../../shell/src/form.rs");

        let page = between(stage, "\"page\" =>", "\"immersive\" =>");
        earlier(page, "page_command", "arm_nav");
        earlier(page, "arm_nav", "web.turn");
        earlier(page, "arm_nav", "turn_pdf");
        earlier(page, "arm_nav", "self.publish()");
        assert!(stage.contains("\"intent\": \"edge\""));

        let wheel_at = web.find("fn take_epub_wheel").expect("epub wheel");
        let wheel = &web[wheel_at..];
        assert!(wheel.contains("NavIntent::Trackpad"));
        earlier(wheel, "arm_nav", "web.follow");
        earlier(wheel, "arm_nav", "web.release_swipe");

        let glide = between(stage, "fn apply_pdf_wheel", "fn note_card");
        assert!(glide.contains("NavIntent::PdfGlide"));
        earlier(glide, "arm_nav", "pdf_glide = Some");

        let scrolled = between(stage, "fn scroll(&mut self", "fn wheel_pdf");
        earlier(scrolled, "arm_nav", "self.publish()");

        let chapter = between(stage, "fn apply_chapter", "fn return_saved_place");
        assert!(chapter.contains("NavIntent::Toc"));
        earlier(chapter, "arm_nav", "web.show_chapter");
        earlier(chapter, "arm_nav", "web.reveal");

        let back = between(stage, "fn return_saved_place", "fn select_phrase");
        earlier(back, "arm_nav", "show_pdf_page");

        let link = between(web, "fn on_link", "fn note_shown");
        assert!(link.contains("NavIntent::Link"));
        earlier(link, "arm_nav", "show_chapter");
        earlier(link, "arm_nav", "chapter.reveal");

        let jump = between(stage, "\"jump\" =>", "fn begin_find");
        assert!(jump.contains("landing"));
        assert!(jump.contains("NavIntent::Bookmark"));
        earlier(jump, "if !landing", "arm_nav");
        earlier(jump, "arm_nav", "jump_place");

        assert!(shell.contains("\"intent\": \"key\""));
        let menu = shell
            .find("invalidate_find_text")
            .expect("the bold menu clears the column before the command");
        assert!(
            shell[menu..].contains("\"bold\""),
            "invalidate_find_text precedes the bold command"
        );

        let bold = between(stage, "\"bold\" =>", "\"epub\" =>");
        earlier(bold, "excerpt_key.clear()", "word_starts_changed");
        earlier(bold, "clear_find()", "word_starts_changed");
        earlier(bold, "set_find_boot", "word_starts_changed");
        earlier(bold, "begin_find", "word_starts_changed");

        assert!(form.contains("nav_epoch()"));
        assert!(form.contains("\"landing\": true"));
        assert!(stage.contains("\"nav\":"));

        for site in stage.match_indices("TurnSource::Restore") {
            let start = site.0.saturating_sub(240);
            let window = &stage[start..site.0];
            assert!(
                !window.contains("arm_nav"),
                "a restore turn is not a user navigation"
            );
        }
    }

    fn between<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
        let at = src.find(start).unwrap_or_else(|| panic!("missing {start}"));
        let rest = &src[at..];
        let stop = rest
            .find(end)
            .unwrap_or_else(|| panic!("missing {end} after {start}"));
        &rest[..stop]
    }

    fn earlier(region: &str, first: &str, second: &str) {
        let left = region
            .find(first)
            .unwrap_or_else(|| panic!("missing {first}"));
        let right = region
            .find(second)
            .unwrap_or_else(|| panic!("missing {second}"));
        assert!(left < right, "{first} should precede {second}");
    }

    #[test]
    fn find_navigation_keys_follow_the_nsh_keymap() {
        use nsh_core::keymap::{KeyBinding, KeyFocus, KeyMatch, Keymap, Keystroke, When};

        let apple = true;
        let strokes = ["shift+return", "mod+g", "shift+mod+g"]
            .map(|chord| Keystroke::parse_with(chord, apple).unwrap());
        let mut editing = KeyFocus::default();
        editing.editing = true;
        let closed = Keymap::new(vec![]);
        for stroke in strokes {
            assert_eq!(closed.find_with(0, stroke, editing, false), KeyMatch::None);
        }
        assert!(!find_navigation_mounted(Mode::Notes, false));
        assert!(!find_navigation_mounted(Mode::Find, true));
        assert!(find_navigation_mounted(Mode::Find, false));
        let open = Keymap::new(
            strokes
                .iter()
                .enumerate()
                .map(|(index, stroke)| {
                    KeyBinding::new((index + 1) as u32, *stroke, When::Always, false)
                })
                .collect(),
        );
        for (index, stroke) in strokes.iter().enumerate() {
            assert_eq!(
                open.find_with(0, *stroke, editing, false),
                KeyMatch::Fire((index + 1) as u32)
            );
        }
        assert!(strokes[0].edits_text());
        assert!(!strokes[1].edits_text());
        assert!(!strokes[2].edits_text());
        let auto = Keymap::new(vec![KeyBinding::new(7, strokes[1], When::Auto, false)]);
        assert_eq!(
            auto.find_with(0, strokes[1], editing, false),
            KeyMatch::Fire(7),
            "⌘G does not edit text, so When::Auto still consumes it in the find field"
        );
        let open_find = Keystroke::parse_with("mod+f", apple).unwrap();
        let window = Keymap::new(vec![KeyBinding::new(4, open_find, When::Always, false)]);
        assert!(!open_find.edits_text());
        assert_eq!(
            window.find_with(0, open_find, editing, false),
            KeyMatch::Fire(4)
        );
    }

    #[test]
    fn the_add_path_stores_an_excerpt_or_a_place() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let marks = Marks::open(file.path()).unwrap();
        let place = Place {
            href: "c5.xhtml".to_string(),
            page: 1,
            offset: 12,
        };
        let column = Column {
            book: "a.epub".to_string(),
            ..Column::default()
        };
        let label = bookmark_label(&place, "我沿着石埠往下走，看见对岸的渔船");
        let column = toggle_mark(&column, &marks, &place, &label, 0).unwrap();
        assert_eq!(column.marks.len(), 1);
        assert!(column.marks[0].label.contains("石埠"));
        let reopened = Marks::open(file.path()).unwrap();
        let rows = reopened.list("a.epub").unwrap();
        assert_eq!(rows[0].label, column.marks[0].label);
        let bare = Place {
            href: String::new(),
            page: 4,
            offset: 0,
        };
        let pdf = toggle_mark(
            &Column {
                book: "a.pdf".to_string(),
                ..Column::default()
            },
            &marks,
            &bare,
            "",
            0,
        )
        .unwrap();
        assert_eq!(pdf.marks[0].label, "第4页");
        let fallback = bookmark_label(&place, "   ");
        assert!(fallback.contains("c5.xhtml"));
        assert!(fallback.contains('1') || fallback.contains('2'));
    }

    #[test]
    fn a_reflow_keeps_the_epub_anchor_and_pdf_pages_stay_distinct() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let marks = Marks::open(file.path()).unwrap();
        let saved = Place {
            href: "chapters/c5.xhtml".to_string(),
            page: 2,
            offset: 18,
        };
        let column = toggle_mark(
            &Column {
                book: "a.epub".to_string(),
                ..Column::default()
            },
            &marks,
            &saved,
            &bookmark_label(&saved, "石埠"),
            0,
        )
        .unwrap();
        let reflow = Place {
            href: "/chapters/c5.xhtml".to_string(),
            page: 9,
            offset: 18,
        };
        assert!(same_place(
            &column.marks[0],
            reflow.page,
            &reflow.href,
            reflow.offset
        ));
        let cancelled = toggle_mark(&column, &marks, &reflow, "", 1_000).unwrap();
        assert!(cancelled.marks.is_empty());
        assert!(cancelled.undo.is_some());
        let restored = undo_delete(&cancelled, &marks).unwrap();
        let again = Marks::open(file.path()).unwrap();
        let rows = again.list("a.epub").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(same_place(&rows[0], 40, &reflow.href, 18));
        let other = Place {
            href: reflow.href.clone(),
            page: 40,
            offset: 19,
        };
        assert!(!same_place(&rows[0], other.page, &other.href, other.offset));
        let two = toggle_mark(&restored, &marks, &other, "另一处", 2_000).unwrap();
        assert_eq!(two.marks.len(), 2);
        let pdf_marks = Marks::open(file.path()).unwrap();
        let page2 = Place {
            href: String::new(),
            page: 2,
            offset: 0,
        };
        let pdf = toggle_mark(
            &Column {
                book: "a.pdf".to_string(),
                ..Column::default()
            },
            &pdf_marks,
            &page2,
            "",
            0,
        )
        .unwrap();
        let page3 = Place {
            href: String::new(),
            page: 3,
            offset: 0,
        };
        assert!(!same_place(&pdf.marks[0], page3.page, "", 0));
        let still = toggle_mark(&pdf, &pdf_marks, &page3, "", 0).unwrap();
        assert_eq!(still.marks.len(), 2);
        let gone = toggle_mark(&still, &pdf_marks, &page2, "", 0).unwrap();
        assert_eq!(gone.marks.len(), 1);
        assert_eq!(gone.marks[0].page, 3);
        let reopened = Marks::open(file.path()).unwrap();
        assert_eq!(reopened.list("a.pdf").unwrap().len(), 1);
    }
}
