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
}
