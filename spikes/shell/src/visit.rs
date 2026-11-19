use std::sync::atomic::{AtomicBool, Ordering};

static ON_LIBRARY: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Library,
    Reading,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaveKey {
    Escape,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Idle,
    Editing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Immersion {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEffect {
    None,
    LeaveBook,
    StayOnBook,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Visit {
    pub surface: Surface,
    pub scroll: f32,
    pub path: String,
    pub page: u32,
}

impl Visit {
    pub fn shelf() -> Self {
        Self {
            surface: Surface::Library,
            scroll: 0.0,
            path: String::new(),
            page: 0,
        }
    }
}

pub fn shelf_origin(offset: f32) -> f32 {
    if offset.is_finite() {
        offset.max(0.0)
    } else {
        0.0
    }
}

pub fn note_scroll(visit: &Visit, offset: f32) -> Visit {
    let mut next = visit.clone();
    if offset.is_finite() {
        next.scroll = shelf_origin(offset);
    }
    next
}

pub fn open_cover(visit: &Visit, path: &str, page: u32) -> Visit {
    let mut next = visit.clone();
    if path.is_empty() {
        return next;
    }
    next.surface = Surface::Reading;
    next.path = path.to_string();
    next.page = page.max(1);
    next
}

pub fn note_page(visit: &Visit, page: u32) -> Visit {
    let mut next = visit.clone();
    if next.surface == Surface::Reading && page >= 1 {
        next.page = page;
    }
    next
}

pub fn leave_to_library(visit: &Visit) -> Visit {
    let mut next = visit.clone();
    next.surface = Surface::Library;
    next
}

pub fn show_library(shell: bool, surface: Surface) -> bool {
    shell && surface == Surface::Library
}

pub fn show_reader_chrome(shell: bool, bars: bool, surface: Surface) -> bool {
    surface == Surface::Reading || (!shell && !bars)
}

pub fn paper_background(shell: bool, surface: Surface) -> bool {
    !shell || surface == Surface::Reading
}

pub fn publish_surface(surface: Surface) {
    ON_LIBRARY.store(surface == Surface::Library, Ordering::Relaxed);
}

pub fn wheel_scrolls_shelf(shell: bool) -> bool {
    shell && ON_LIBRARY.load(Ordering::Relaxed)
}

pub const OPEN_REFUSAL: &str = "打不开这个文件";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileOpen {
    Read { path: String, page: u32 },
    Refuse,
}

pub fn prefix_is_book(pdf: bool, bytes: &[u8]) -> bool {
    if pdf {
        bytes.starts_with(b"%PDF")
    } else {
        bytes.starts_with(b"PK")
    }
}

pub fn file_open(openable: bool, path: &str, page: u32) -> FileOpen {
    if !openable || path.is_empty() {
        FileOpen::Refuse
    } else {
        FileOpen::Read {
            path: path.to_string(),
            page: page.max(1),
        }
    }
}

pub fn apply_file_open(visit: &Visit, open: &FileOpen) -> Visit {
    match open {
        FileOpen::Refuse => visit.clone(),
        FileOpen::Read { path, page } => open_cover(visit, path, *page),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageOpen {
    Command,
    Queue,
    None,
}

pub fn stage_open(open: &FileOpen, stage_ready: bool) -> StageOpen {
    match open {
        FileOpen::Refuse => StageOpen::None,
        FileOpen::Read { .. } if stage_ready => StageOpen::Command,
        FileOpen::Read { .. } => StageOpen::Queue,
    }
}

pub fn fresh_arrival(seq: u64, seen: u64) -> bool {
    seq != 0 && seq != seen
}

/// The sentence stored with a page. A new page replaces it. The same page
/// keeps a stored sentence when the new sample is empty.
pub fn next_sentence(stored_page: u32, stored_offset: u32, page: u32, offset: u32) -> (u32, u32) {
    if page == 0 {
        return (stored_page, stored_offset);
    }
    if page != stored_page {
        return (page, offset);
    }
    if offset == 0 {
        return (stored_page, stored_offset);
    }
    (page, offset)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Restore {
    pub write: bool,
    pub want: Option<u32>,
    pub hold: Option<u32>,
}

/// A stored sentence stays until reveal reports that offset. An opening-page
/// sample, including a nonzero one, is not that report. A missed target keeps
/// the stored sentence until the reader leaves that page.
pub fn step_restore(
    pending: bool,
    want: Option<u32>,
    hold: Option<u32>,
    shown: u32,
    pinned: Option<u32>,
    missed: bool,
) -> Restore {
    if pending {
        return Restore {
            write: false,
            want,
            hold,
        };
    }
    if let Some(offset) = want {
        if pinned != Some(offset) {
            return Restore {
                write: false,
                want,
                hold,
            };
        }
        if missed {
            return Restore {
                write: false,
                want: None,
                hold: Some(shown),
            };
        }
        return Restore {
            write: true,
            want: None,
            hold: None,
        };
    }
    if hold == Some(shown) {
        return Restore {
            write: false,
            want: None,
            hold,
        };
    }
    Restore {
        write: true,
        want: None,
        hold: None,
    }
}

pub fn page_key_delta(shell: bool, surface: Surface, forward: bool) -> i32 {
    if show_library(shell, surface) {
        0
    } else if forward {
        1
    } else {
        -1
    }
}

/// Outer fraction of the page that a click turns. The middle stays.
pub const PAGE_EDGE: f64 = 0.25;

/// A move of this many points is a drag, not a click. The page script uses it.
pub const CLICK_SLOP: f64 = 3.0;

pub fn click_is_drag(dx: f64, dy: f64) -> bool {
    dx.abs() >= CLICK_SLOP || dy.abs() >= CLICK_SLOP
}

pub fn point_in_box(x: f64, y: f64, left: f64, top: f64, width: f64, height: f64) -> bool {
    width > 0.0 && height > 0.0 && x >= left && y >= top && x <= left + width && y <= top + height
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageEdge {
    Back,
    Forward,
    None,
}

/// A click on the left or right of the page rectangle turns one page.
/// A click in the middle, or outside the rectangle, does not.
pub fn page_edge(x: f64, y: f64, left: f64, top: f64, width: f64, height: f64) -> PageEdge {
    if !(width > 0.0 && height > 0.0) {
        return PageEdge::None;
    }
    let at_x = x - left;
    let at_y = y - top;
    if at_x < 0.0 || at_y < 0.0 || at_x > width || at_y > height {
        return PageEdge::None;
    }
    let band = width * PAGE_EDGE;
    if at_x < band {
        PageEdge::Back
    } else if at_x > width - band {
        PageEdge::Forward
    } else {
        PageEdge::None
    }
}

pub fn step_index(index: usize, count: usize, delta: i64) -> usize {
    if count == 0 || delta == 0 {
        return index.min(count.saturating_sub(1));
    }
    let index = index.min(count - 1);
    if delta < 0 {
        index.saturating_sub(1)
    } else {
        (index + 1).min(count - 1)
    }
}

pub fn leave_key(key: LeaveKey, focus: Focus, immersion: Immersion, surface: Surface) -> KeyEffect {
    if focus == Focus::Editing {
        return KeyEffect::None;
    }
    if immersion == Immersion::On {
        return match key {
            LeaveKey::Escape => KeyEffect::StayOnBook,
            LeaveKey::Back => KeyEffect::None,
        };
    }
    if surface == Surface::Reading {
        return KeyEffect::LeaveBook;
    }
    KeyEffect::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scrolled(offset: f32) -> Visit {
        note_scroll(&Visit::shelf(), offset)
    }

    #[test]
    fn epub_open_keeps_the_shelf_scroll_and_returns_the_page() {
        let opened = open_cover(&scrolled(240.0), "/books/plain-essay.epub", 4);
        assert_eq!(opened.surface, Surface::Reading);
        assert_eq!(opened.scroll, 240.0);
        assert_eq!(opened.page, 4);
        let turned = note_page(&opened, 7);
        let back = leave_to_library(&turned);
        assert_eq!(back.surface, Surface::Library);
        assert_eq!(back.scroll, 240.0);
        assert_eq!(back.page, 7);
        assert_eq!(back.path, "/books/plain-essay.epub");
    }

    #[test]
    fn pdf_open_keeps_the_shelf_scroll_and_returns_the_page() {
        let opened = open_cover(&scrolled(80.0), "/books/field-guide.pdf", 2);
        assert_eq!(opened.surface, Surface::Reading);
        assert_eq!(opened.path, "/books/field-guide.pdf");
        assert_eq!(opened.page, 2);
        let back = leave_to_library(&note_page(&opened, 5));
        assert_eq!(back.surface, Surface::Library);
        assert_eq!(back.scroll, 80.0);
        assert_eq!(back.page, 5);
    }

    #[test]
    fn leave_keys_skip_editing_and_immersive() {
        let reading = Surface::Reading;
        let library = Surface::Library;
        assert_eq!(
            leave_key(LeaveKey::Escape, Focus::Idle, Immersion::Off, reading),
            KeyEffect::LeaveBook
        );
        assert_eq!(
            leave_key(LeaveKey::Back, Focus::Idle, Immersion::Off, reading),
            KeyEffect::LeaveBook
        );
        assert_eq!(
            leave_key(LeaveKey::Escape, Focus::Editing, Immersion::Off, reading),
            KeyEffect::None
        );
        assert_eq!(
            leave_key(LeaveKey::Back, Focus::Editing, Immersion::Off, reading),
            KeyEffect::None
        );
        assert_eq!(
            leave_key(LeaveKey::Escape, Focus::Idle, Immersion::On, reading),
            KeyEffect::StayOnBook
        );
        assert_eq!(
            leave_key(LeaveKey::Back, Focus::Idle, Immersion::On, reading),
            KeyEffect::None
        );
        assert_eq!(
            leave_key(LeaveKey::Escape, Focus::Idle, Immersion::Off, library),
            KeyEffect::None
        );
        assert_eq!(
            leave_key(LeaveKey::Back, Focus::Idle, Immersion::Off, library),
            KeyEffect::None
        );
    }

    #[test]
    fn an_empty_path_and_a_bad_offset_do_not_move_the_shelf() {
        let shelf = scrolled(12.0);
        assert_eq!(open_cover(&shelf, "", 3), shelf);
        assert_eq!(note_page(&shelf, 3).page, 0);
        assert_eq!(note_scroll(&shelf, f32::NAN).scroll, 12.0);
        assert_eq!(shelf_origin(-4.0), 0.0);
        assert_eq!(open_cover(&shelf, "/books/a.epub", 0).page, 1);
    }

    #[test]
    fn the_shell_shows_the_library_or_the_existing_reader() {
        assert!(show_library(true, Surface::Library));
        assert!(!show_library(true, Surface::Reading));
        assert!(!show_library(false, Surface::Library));
        assert!(show_reader_chrome(true, true, Surface::Reading));
        assert!(!show_reader_chrome(true, true, Surface::Library));
        assert!(show_reader_chrome(false, false, Surface::Library));
        assert!(!paper_background(true, Surface::Library));
        assert!(paper_background(true, Surface::Reading));
        assert!(paper_background(false, Surface::Library));
    }

    #[test]
    fn a_wheel_on_the_reader_reaches_the_page_and_returns_to_the_shelf() {
        publish_surface(Surface::Library);
        assert!(wheel_scrolls_shelf(true));
        assert!(!wheel_scrolls_shelf(false));
        publish_surface(Surface::Reading);
        assert!(!wheel_scrolls_shelf(true));
        publish_surface(Surface::Library);
        assert!(wheel_scrolls_shelf(true));
    }

    fn fixture(name: &str) -> Vec<u8> {
        for path in [
            format!("spikes/library/fixtures/apple/{name}"),
            format!("library/fixtures/apple/{name}"),
            format!("../library/fixtures/apple/{name}"),
        ] {
            if let Ok(bytes) = std::fs::read(&path) {
                return bytes;
            }
        }
        panic!("missing fixture {name}");
    }

    #[test]
    fn a_file_open_while_running_switches_books_and_refuses_a_bad_file() {
        let essay = fixture("Plain Essay.epub");
        let guide = fixture("Field Guide.pdf");
        assert!(prefix_is_book(false, &essay));
        assert!(prefix_is_book(true, &guide));
        assert!(!prefix_is_book(true, b"not a pdf"));
        assert!(!prefix_is_book(false, b"not a zip"));

        let shelf = note_scroll(&Visit::shelf(), 240.0);
        let opened = apply_file_open(&shelf, &file_open(true, "/books/plain-essay.epub", 4));
        assert_eq!(opened.surface, Surface::Reading);
        assert_eq!(opened.path, "/books/plain-essay.epub");
        assert_eq!(opened.page, 4);
        assert_eq!(opened.scroll, 240.0);

        let switched = apply_file_open(&opened, &file_open(true, "/books/field-guide.pdf", 2));
        assert_eq!(switched.surface, Surface::Reading);
        assert_eq!(switched.path, "/books/field-guide.pdf");
        assert_eq!(switched.page, 2);
        assert_eq!(switched.scroll, 240.0);

        let stayed = apply_file_open(&switched, &file_open(false, "/tmp/notes.txt", 9));
        assert_eq!(stayed, switched);
        assert_eq!(apply_file_open(&shelf, &file_open(false, "", 1)), shelf);
        assert_eq!(OPEN_REFUSAL, "打不开这个文件");

        let refused = file_open(false, "/tmp/notes.pdf", 1);
        let accepted = file_open(true, "/books/field-guide.pdf", 2);
        assert_eq!(stage_open(&refused, true), StageOpen::None);
        assert_eq!(stage_open(&refused, false), StageOpen::None);
        assert_eq!(stage_open(&accepted, true), StageOpen::Command);
        assert_eq!(stage_open(&accepted, false), StageOpen::Queue);
        assert!(fresh_arrival(1, 0));
        assert!(!fresh_arrival(1, 1));
        assert!(!fresh_arrival(0, 0));
        assert!(fresh_arrival(2, 1));
    }

    #[test]
    fn a_page_key_turns_one_page_and_stops_at_the_ends() {
        assert_eq!(page_key_delta(true, Surface::Library, true), 0);
        assert_eq!(page_key_delta(true, Surface::Library, false), 0);
        assert_eq!(page_key_delta(false, Surface::Library, true), 1);
        assert_eq!(page_key_delta(false, Surface::Library, false), -1);
        assert_eq!(page_key_delta(true, Surface::Reading, true), 1);
        assert_eq!(page_key_delta(true, Surface::Reading, false), -1);

        let count = 10;
        let mut index = 3;
        index = step_index(
            index,
            count,
            page_key_delta(true, Surface::Reading, true) as i64,
        );
        assert_eq!(index, 4);
        index = step_index(
            index,
            count,
            page_key_delta(true, Surface::Reading, false) as i64,
        );
        assert_eq!(index, 3);
        assert_eq!(step_index(0, count, -1), 0);
        assert_eq!(step_index(9, count, 1), 9);
        assert_eq!(step_index(0, 0, 1), 0);
    }

    #[test]
    fn a_reopen_keeps_the_sentence_on_the_same_page() {
        assert_eq!(next_sentence(4, 800, 4, 0), (4, 800));
        assert_eq!(next_sentence(4, 800, 4, 860), (4, 860));
        assert_eq!(next_sentence(4, 800, 5, 20), (5, 20));
        assert_eq!(next_sentence(4, 800, 5, 0), (5, 0));
        assert_eq!(next_sentence(4, 800, 0, 20), (4, 800));
        let pending = step_restore(true, None, None, 1, None, false);
        assert!(!pending.write);
        let opening = step_restore(false, Some(800), None, 1, Some(12), false);
        assert!(!opening.write);
        assert_eq!(opening.want, Some(800));
        let landed = step_restore(false, Some(800), None, 3, Some(800), false);
        assert!(landed.write);
        assert_eq!(landed.want, None);
        let missed = step_restore(false, Some(800), None, 1, Some(800), true);
        assert!(!missed.write);
        assert_eq!(missed.hold, Some(1));
        let still = step_restore(false, None, Some(1), 1, None, false);
        assert!(!still.write);
        let turned = step_restore(false, None, Some(1), 2, None, false);
        assert!(turned.write);
        assert_eq!(turned.hold, None);
    }

    #[test]
    fn a_click_on_the_page_edge_turns_and_the_middle_does_not() {
        let (left, top, width, height) = (100.0, 20.0, 400.0, 500.0);
        assert_eq!(
            page_edge(100.0, 20.0, left, top, width, height),
            PageEdge::Back
        );
        assert_eq!(
            page_edge(199.0, 250.0, left, top, width, height),
            PageEdge::Back
        );
        assert_eq!(
            page_edge(200.0, 250.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(400.0, 250.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(401.0, 519.0, left, top, width, height),
            PageEdge::Forward
        );
        assert_eq!(
            page_edge(500.0, 520.0, left, top, width, height),
            PageEdge::Forward
        );
        assert_eq!(
            page_edge(99.0, 250.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(501.0, 250.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(150.0, 19.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(450.0, 521.0, left, top, width, height),
            PageEdge::None
        );
        assert_eq!(
            page_edge(150.0, 250.0, left, top, 0.0, height),
            PageEdge::None
        );
        assert!(!click_is_drag(2.9, 2.9));
        assert!(click_is_drag(3.0, 0.0));
        assert!(click_is_drag(0.0, 3.0));
        let (fx, fy, fw, fh) = (0.0, 480.0, 500.0, 40.0);
        assert!(point_in_box(10.0, 500.0, fx, fy, fw, fh));
        assert!(point_in_box(450.0, 500.0, fx, fy, fw, fh));
        assert!(point_in_box(500.0, 520.0, fx, fy, fw, fh));
        assert!(!point_in_box(10.0, 479.0, fx, fy, fw, fh));
        assert_eq!(
            page_edge(450.0, 250.0, left, top, width, height),
            PageEdge::Forward
        );
        assert!(click_is_drag(3.0, 0.0));
    }
}
