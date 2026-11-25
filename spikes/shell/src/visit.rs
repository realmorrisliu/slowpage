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

/// Who consumes one scroll-wheel event. The shelf flag selects the shelf.
/// It does not decide whether the monitor is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelOwner {
    Shelf,
    Page,
    Through,
}

pub fn wheel_owner(scrolls_shelf: bool, page_took_it: bool) -> WheelOwner {
    if scrolls_shelf {
        WheelOwner::Shelf
    } else if page_took_it {
        WheelOwner::Page
    } else {
        WheelOwner::Through
    }
}

/// Who may see a wheel before any page gesture starts. `Margin` means the
/// card column owns it: the caller must not ask the page to turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelStep {
    Shelf,
    Page,
    Margin,
}

pub fn wheel_step(margin_claims: bool, scrolls_shelf: bool) -> WheelStep {
    if scrolls_shelf {
        WheelStep::Shelf
    } else if margin_claims {
        WheelStep::Margin
    } else {
        WheelStep::Page
    }
}

/// The PDF page the margin shows. While a turn is moving, this is the
/// departing page. `None` is an empty document.
pub fn margin_pdf_index(
    count: usize,
    index: usize,
    departing: usize,
    moving: bool,
) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let index = index.min(count - 1);
    Some(if moving {
        departing.min(count - 1)
    } else {
        index
    })
}

pub fn margin_page_number(index: usize) -> u32 {
    (index as u32).saturating_add(1)
}

/// Measured EPUB selection. Offsets are Unicode scalars, not the UTF-16
/// indexes the page script still uses to paint and resume.
pub struct SelectionMeasure {
    pub page: u32,
    pub start: u32,
    pub end: u32,
    pub text: String,
    pub rects: Vec<(f64, f64, f64, f64)>,
    pub text_right: f64,
}

pub fn selection_measure(value: &serde_json::Value) -> Option<SelectionMeasure> {
    let page = value.get("page")?.as_u64()? as u32;
    let start = value.get("scalar_start")?.as_u64()? as u32;
    let end = value.get("scalar_end")?.as_u64()? as u32;
    let text = value.get("text")?.as_str()?.to_string();
    let text_right = value.get("text_right")?.as_f64()?;
    let rects = value
        .get("rects")?
        .as_array()?
        .iter()
        .filter_map(|rect| {
            Some((
                rect.get("x")?.as_f64()?,
                rect.get("y")?.as_f64()?,
                rect.get("w")?.as_f64()?,
                rect.get("h")?.as_f64()?,
            ))
        })
        .collect();
    Some(SelectionMeasure {
        page,
        start,
        end,
        text,
        rects,
        text_right,
    })
}

pub fn fresh_measures(value: &serde_json::Value) -> Vec<spike_library::margin::FreshMeasure> {
    let Some(rows) = value.as_array() else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let id = row.get("id")?.as_i64()?;
            let page = row.get("page")?.as_u64()? as u32;
            if page == 0 {
                return None;
            }
            let text_right = row.get("text_right")?.as_f64()?;
            if !text_right.is_finite() || text_right <= 0.0 {
                return None;
            }
            let quads = row.get("rects")?.as_array()?.iter().filter_map(|rect| {
                let x = rect.get("x")?.as_f64()?;
                let y = rect.get("y")?.as_f64()?;
                let w = rect.get("w")?.as_f64()?;
                let h = rect.get("h")?.as_f64()?;
                if ![x, y, w, h].iter().all(|n| n.is_finite()) || w <= 0.0 || h <= 0.0 {
                    return None;
                }
                Some(spike_library::margin::Quad { x, y, w, h })
            });
            let quads: Vec<_> = quads.collect();
            if quads.is_empty() {
                return None;
            }
            Some(spike_library::margin::FreshMeasure {
                id,
                page,
                quads,
                text_right,
            })
        })
        .collect()
}

pub fn passage_value(passage: Option<&spike_library::margin::Passage>) -> serde_json::Value {
    let Some(passage) = passage else {
        return serde_json::Value::Null;
    };
    serde_json::json!({
        "book": passage.book,
        "gen": passage.gen,
        "page": passage.page,
        "spine": passage.spine,
        "start": passage.start,
        "end": passage.end,
        "text": passage.text,
        "quads": passage
            .quads
            .iter()
            .map(|quad| [quad.x, quad.y, quad.w, quad.h])
            .collect::<Vec<_>>(),
        "text_right": passage.text_right,
    })
}

pub fn passage_from_value(value: &serde_json::Value) -> Option<spike_library::margin::Passage> {
    if value.is_null() {
        return None;
    }
    let quads = value.get("quads")?.as_array().map(|rows| {
        rows.iter()
            .filter_map(|quad| {
                let row = quad.as_array()?;
                if row.len() != 4 {
                    return None;
                }
                Some(spike_library::margin::Quad {
                    x: row[0].as_f64()?,
                    y: row[1].as_f64()?,
                    w: row[2].as_f64()?,
                    h: row[3].as_f64()?,
                })
            })
            .collect()
    })?;
    spike_library::margin::passage_from(spike_library::margin::Selection {
        book: value.get("book")?.as_str()?.to_string(),
        gen: value.get("gen")?.as_u64()?,
        page: value.get("page")?.as_u64()? as u32,
        spine: value
            .get("spine")
            .and_then(|spine| spine.as_str())
            .unwrap_or("")
            .to_string(),
        start: value.get("start")?.as_u64()? as u32,
        end: value.get("end")?.as_u64()? as u32,
        text: value.get("text")?.as_str()?.to_string(),
        quads,
        text_right: value.get("text_right")?.as_f64()?,
    })
}

/// The scroll monitor is installed for a direct file as well as the shelf.
/// `shell` is the shelf flag. It does not gate installation.
pub fn arms_page_wheel(shell: bool) -> bool {
    let _ = shell;
    true
}

/// What `place_page` does with a gesture that may still be painting.
/// The same fitted frame on the same stage leaves the release alone.
/// A stage change cancels it even when the fitted integers and zoom do not move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceStep {
    Hold,
    Cancel { reflow: bool },
}

pub fn place_step(fitted_same: bool, zoom_same: bool, stage_same: bool) -> PlaceStep {
    if fitted_same && zoom_same && stage_same {
        PlaceStep::Hold
    } else if fitted_same && zoom_same {
        PlaceStep::Cancel { reflow: false }
    } else {
        PlaceStep::Cancel { reflow: true }
    }
}

pub fn on_library() -> bool {
    ON_LIBRARY.load(Ordering::Relaxed)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingPlace {
    pub href: String,
    pub page: u32,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterMark {
    pub title: String,
    pub href: String,
    pub offset: u32,
    pub page: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChapterMove {
    Stay,
    Reveal { offset: u32 },
    Chapter { index: usize, offset: u32 },
    Page { page: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterStep {
    pub go: ChapterMove,
    pub back: Option<ReadingPlace>,
}

pub fn contents_marks(title: &str, spine: &[String]) -> Vec<ChapterMark> {
    spine
        .iter()
        .map(|href| ChapterMark {
            title: chapter_title(title, spine.len(), href),
            href: href.clone(),
            offset: 0,
            page: 0,
        })
        .collect()
}

fn chapter_title(title: &str, count: usize, href: &str) -> String {
    if count == 1 && !title.is_empty() {
        return title.to_string();
    }
    href.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(href)
        .to_string()
}

fn move_toward(here: &ReadingPlace, mark: &ChapterMark, spine: &[String]) -> ChapterMove {
    if mark.href.is_empty() {
        if mark.page >= 1 && mark.page != here.page {
            ChapterMove::Page { page: mark.page }
        } else {
            ChapterMove::Stay
        }
    } else if mark.href == here.href {
        if mark.offset == here.offset {
            ChapterMove::Stay
        } else {
            ChapterMove::Reveal {
                offset: mark.offset,
            }
        }
    } else if let Some(index) = spine.iter().position(|href| href == &mark.href) {
        ChapterMove::Chapter {
            index,
            offset: mark.offset,
        }
    } else {
        ChapterMove::Stay
    }
}

pub fn chapter_jump(
    here: &ReadingPlace,
    marks: &[ChapterMark],
    index: usize,
    spine: &[String],
    back: Option<ReadingPlace>,
) -> ChapterStep {
    let Some(mark) = marks.get(index) else {
        return ChapterStep {
            go: ChapterMove::Stay,
            back,
        };
    };
    let go = move_toward(here, mark, spine);
    if matches!(go, ChapterMove::Stay) {
        return ChapterStep { go, back };
    }
    ChapterStep {
        go,
        back: Some(here.clone()),
    }
}

/// A bookmark or search jump. The return slot is filled once, then kept.
pub fn place_jump(
    here: &ReadingPlace,
    dest: &ReadingPlace,
    spine: &[String],
    back: Option<ReadingPlace>,
) -> ChapterStep {
    let mark = ChapterMark {
        title: String::new(),
        href: dest.href.clone(),
        offset: dest.offset,
        page: dest.page,
    };
    let go = move_toward(here, &mark, spine);
    if matches!(go, ChapterMove::Stay) {
        return ChapterStep { go, back };
    }
    let back = if back.is_none() {
        Some(here.clone())
    } else {
        back
    };
    ChapterStep { go, back }
}

pub fn chapter_return(
    here: &ReadingPlace,
    back: Option<ReadingPlace>,
    spine: &[String],
) -> ChapterStep {
    let Some(place) = back else {
        return ChapterStep {
            go: ChapterMove::Stay,
            back: None,
        };
    };
    let mark = ChapterMark {
        title: String::new(),
        href: place.href.clone(),
        offset: place.offset,
        page: place.page,
    };
    let go = move_toward(here, &mark, spine);
    let already = place.href == here.href && place.offset == here.offset
        || place.href.is_empty() && place.page == here.page;
    let keep_missing = matches!(go, ChapterMove::Stay) && !already;
    let back = if keep_missing { Some(place) } else { None };
    ChapterStep { go, back }
}

/// The stored sentence is the place. A jump that already has an offset for
/// the href on screen is that sentence. A view can change before the
/// sentence does. That view is not a sentence, and it is not the return.
fn shown_place(
    shown_href: &str,
    shown_page: u32,
    saved_href: &str,
    saved_page: u32,
    saved_offset: u32,
    jump_href: &str,
    jump_offset: Option<u32>,
    awaiting: Option<u32>,
) -> ReadingPlace {
    if jump_href == shown_href {
        if let Some(offset) = jump_offset {
            return ReadingPlace {
                href: shown_href.to_string(),
                page: 1,
                offset,
            };
        }
    }
    if !saved_href.is_empty() {
        let offset = if saved_href == shown_href {
            awaiting.unwrap_or(saved_offset)
        } else {
            saved_offset
        };
        return ReadingPlace {
            href: saved_href.to_string(),
            page: saved_page.max(1),
            offset,
        };
    }
    ReadingPlace {
        href: shown_href.to_string(),
        page: shown_page.max(1),
        offset: 0,
    }
}

pub struct NavIn<'a> {
    pub shown_href: &'a str,
    pub shown_page: u32,
    pub saved_href: &'a str,
    pub saved_page: u32,
    pub saved_offset: u32,
    pub jump_href: &'a str,
    pub jump_offset: Option<u32>,
    pub awaiting: Option<u32>,
    pub pending: u32,
    pub back: Option<ReadingPlace>,
    pub marks: &'a [ChapterMark],
    pub spine: &'a [String],
    pub pick: isize,
}

pub struct NavOut {
    pub go: ChapterMove,
    pub back: Option<ReadingPlace>,
    pub pending: u32,
    pub jump_href: String,
    pub jump_offset: Option<u32>,
    pub awaiting: Option<u32>,
    pub saved_href: String,
    pub saved_page: u32,
    pub saved_offset: u32,
    pub record: bool,
}

/// `pickChapter` runs this before reveal or `show_chapter`. A move replaces
/// the open resume. The saved row becomes the place that move requested.
pub fn navigate(nav: NavIn<'_>) -> NavOut {
    let here = shown_place(
        nav.shown_href,
        nav.shown_page,
        nav.saved_href,
        nav.saved_page,
        nav.saved_offset,
        nav.jump_href,
        nav.jump_offset,
        nav.awaiting,
    );
    let back = nav.back.clone();
    let mut step = if nav.pick < 0 {
        chapter_return(&here, back.clone(), nav.spine)
    } else {
        chapter_jump(&here, nav.marks, nav.pick as usize, nav.spine, back.clone())
    };
    // Reveal and Stay compare the stored sentence. They can run only when
    // that sentence is the document on screen. Otherwise the choice loads
    // the chapter it named.
    let chosen = if nav.pick < 0 {
        back.as_ref()
            .map(|place| (place.href.clone(), place.offset))
    } else {
        nav.marks.get(nav.pick as usize).and_then(|mark| {
            if mark.href.is_empty() {
                None
            } else {
                Some((mark.href.clone(), mark.offset))
            }
        })
    };
    if let Some((href, offset)) = chosen {
        if !nav.shown_href.is_empty()
            && spine_key(nav.shown_href) != spine_key(&href)
            && matches!(step.go, ChapterMove::Stay | ChapterMove::Reveal { .. })
        {
            if let Some(index) = spine_pos(nav.spine, &href) {
                step.go = ChapterMove::Chapter { index, offset };
            }
        }
    }
    let pending = if matches!(step.go, ChapterMove::Stay) {
        nav.pending
    } else {
        0
    };
    let saved_href = nav.saved_href.to_string();
    let mut saved_page = nav.saved_page;
    let mut saved_offset = nav.saved_offset;
    let mut jump_href = nav.jump_href.to_string();
    let mut jump_offset = nav.jump_offset;
    let mut awaiting = nav.awaiting;
    let mut record = false;
    match &step.go {
        ChapterMove::Stay => {}
        ChapterMove::Reveal { offset } => {
            let offset = *offset;
            jump_href.clear();
            jump_offset = None;
            awaiting = Some(offset);
            if saved_href != nav.shown_href || saved_offset != offset {
                saved_page = if saved_href == nav.shown_href {
                    saved_page.max(1)
                } else {
                    1
                };
                saved_offset = offset;
                record = true;
            }
        }
        ChapterMove::Chapter { index, offset } => {
            let offset = *offset;
            if let Some(href) = nav.spine.get(*index) {
                jump_href = href.clone();
                jump_offset = Some(offset);
                awaiting = None;
                if saved_href != *href || saved_offset != offset {
                    saved_page = 1;
                    saved_offset = offset;
                    record = true;
                }
            }
        }
        ChapterMove::Page { .. } => {}
    }
    let saved_href = match &step.go {
        ChapterMove::Chapter { index, .. } => nav.spine.get(*index).cloned().unwrap_or(saved_href),
        ChapterMove::Reveal { .. } => nav.shown_href.to_string(),
        _ => saved_href,
    };
    NavOut {
        go: step.go,
        back: step.back,
        pending,
        jump_href,
        jump_offset,
        awaiting,
        saved_href,
        saved_page,
        saved_offset,
        record,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenedPlace {
    pub index: usize,
    pub page: u32,
    pub offset: u32,
}

pub fn opened_place(spine: &[String], href: &str, page: u32, offset: u32) -> OpenedPlace {
    let page = page.max(1);
    if href.is_empty() {
        return OpenedPlace {
            index: 0,
            page,
            offset,
        };
    }
    if let Some(index) = spine.iter().position(|item| item == href) {
        return OpenedPlace {
            index,
            page,
            offset,
        };
    }
    OpenedPlace {
        index: 0,
        page: 1,
        offset: 0,
    }
}

pub fn spine_index(spine: &[String], href: &str) -> usize {
    opened_place(spine, href, 1, 0).index
}

pub fn changed_chapter(stored_href: &str, href: &str) -> bool {
    !href.is_empty() && href != stored_href
}

/// Horizontal travel, in points, that commits one EPUB page.
/// This is the page script's old `SWIPE` value. The finger path uses it once.
pub const SWIPE_POINTS: f64 = 48.0;

/// AppKit `NSEventPhase` bits. `phase` and `momentumPhase` are separate.
const PHASE_BEGAN: u64 = 1;
const PHASE_STATIONARY: u64 = 2;
const PHASE_CHANGED: u64 = 4;
const PHASE_ENDED: u64 = 8;
const PHASE_CANCELLED: u64 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    None,
    Down,
    Up,
    Cancelled,
}

/// `phase` while the fingers are down. Momentum is not a finger.
pub fn finger_bits(phase: u64) -> Finger {
    if phase & PHASE_CANCELLED != 0 {
        Finger::Cancelled
    } else if phase & PHASE_ENDED != 0 {
        Finger::Up
    } else if phase & (PHASE_BEGAN | PHASE_STATIONARY | PHASE_CHANGED) != 0 {
        Finger::Down
    } else {
        Finger::None
    }
}

/// Precise trackpad deltas are already in points. A line-mode wheel uses the
/// same 16 point step the shelf uses when `scrollingDeltaY` is empty.
pub fn wheel_points(precise: bool, scrolling: f64, lines: f64) -> f64 {
    if precise || scrolling.abs() >= 0.01 {
        scrolling
    } else {
        lines * 16.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelTick {
    pub finger: Finger,
    pub momentum: bool,
    /// Positive moves toward the next page. That is AppKit `scrollingDeltaX`:
    /// positive is to the right, which is the sign the old wheel handler used.
    pub dx: f64,
    pub dy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelMove {
    /// Not an EPUB page gesture. The caller lets the event through.
    Pass,
    /// Eat the event. Do not turn and do not write progress.
    Swallow,
    /// Still accumulating a mouse-wheel notch. Do not write progress.
    Hold,
    /// Paint this offset. Do not write progress.
    Follow(f64),
    /// Seek exactly one page. This is the only effect that changes progress.
    Commit(i32),
    /// The lift asked for a page that is not there. Paint the ease. Do not write progress.
    Ease(i32),
    /// Return to the page the gesture started on. Do not write progress.
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Swipe {
    travel: f64,
    live: bool,
    notch: f64,
    /// A key, chapter, resize, or new book took the gesture. Ignore the rest
    /// of this finger, including the lift, so that lift cannot turn again.
    ignore: bool,
    /// A commit or a return glide has been handed to the page and has not
    /// been dropped. The finger may already be up. A matching commit post
    /// clears this. Navigation clears it. A new finger does not.
    release: bool,
    next_id: u64,
    /// Tokens at or below this were retired by navigation.
    floor: u64,
    /// The newest commit whose post was stored. An older post does not
    /// move the saved page backward.
    last_applied: u64,
    /// The glide the page is running. Its post carries this token.
    flying: Option<u64>,
    /// The one queued page. A newer commit replaces it. A key that takes
    /// the slot retires it. There is no window of old commits.
    queued: Option<u64>,
    /// The commit just handed to the page. The wheel command reads it once.
    last_commit: Option<u64>,
}

impl Default for Swipe {
    fn default() -> Self {
        Self {
            travel: 0.0,
            live: false,
            notch: 0.0,
            ignore: false,
            release: false,
            next_id: 0,
            floor: 0,
            last_applied: 0,
            flying: None,
            queued: None,
            last_commit: None,
        }
    }
}

impl Swipe {
    /// Drop a gesture that resize, a new book, a chapter, or a key interrupted.
    /// The next lift must not turn.
    pub fn abandon(&mut self) -> bool {
        let was = self.live;
        // A finger that started during an open commit is not that commit.
        // Dropping the page paint would cancel the page already handed over.
        let drop_paint = was && self.flying.is_none();
        self.travel = 0.0;
        self.notch = 0.0;
        self.live = false;
        if was {
            self.ignore = true;
        }
        drop_paint
    }

    pub fn commit_token(&self) -> Option<u64> {
        self.last_commit
    }

    /// The page accepted or refused the token it was just given.
    /// `glide` and `queued` are the only answers that keep it.
    pub fn note_answer(&mut self, token: u64, answer: &str) {
        if token == 0 || token <= self.floor {
            return;
        }
        match answer {
            "glide" => {
                if self.queued == Some(token) {
                    self.queued = None;
                }
                if self.flying.is_none() || self.flying == Some(token) {
                    self.flying = Some(token);
                } else {
                    self.queued = Some(token);
                }
            }
            "queued" => {
                if self.flying == Some(token) {
                    self.flying = None;
                }
                self.queued = Some(token);
            }
            _ => {
                if self.flying == Some(token) {
                    self.flying = None;
                }
                if self.queued == Some(token) {
                    self.queued = None;
                }
            }
        }
    }

    /// The commit glide finished and reported `token`. A token the page
    /// refused, replaced, or navigation retired does not change the stored page.
    /// Either of the two slots may arrive first. A repeat does not apply.
    pub fn take_commit_post(&mut self, token: u64) -> bool {
        let held = self.holds(token);
        if held {
            self.clear_token(token);
        }
        if !held || token <= self.last_applied {
            return false;
        }
        self.last_applied = token;
        true
    }

    /// The page dropped this token without turning for it. A key taking the
    /// one queue slot uses this. The column is not moved here.
    pub fn retire(&mut self, token: u64) {
        self.clear_token(token);
    }

    fn holds(&self, token: u64) -> bool {
        token > self.floor && (self.flying == Some(token) || self.queued == Some(token))
    }

    fn clear_token(&mut self, token: u64) {
        if self.flying == Some(token) {
            self.flying = None;
        }
        if self.queued == Some(token) {
            self.queued = None;
        }
        if self.flying.is_none() && self.queued.is_none() {
            self.release = false;
            self.last_commit = None;
        }
    }

    /// The page is still painting the finger-up glide. A key does not use this.
    pub fn release_open(&self) -> bool {
        self.release
    }

    /// Drop a follow or the finger-up glide. A chapter, a reveal, a return,
    /// a resize, a reopen, or another book calls this. A key does not.
    pub fn cancel_release(&mut self) -> bool {
        let drop = self.live || self.release || self.flying.is_some() || self.queued.is_some();
        let was = self.live;
        self.travel = 0.0;
        self.notch = 0.0;
        self.live = false;
        self.release = false;
        self.floor = self.next_id;
        self.flying = None;
        self.queued = None;
        self.last_commit = None;
        if was {
            self.ignore = true;
        }
        drop
    }

    fn release_finger(&mut self) {
        let next_id = self.next_id;
        let floor = self.floor;
        let last_applied = self.last_applied;
        let flying = self.flying;
        let queued = self.queued;
        let last_commit = self.last_commit;
        let release = self.release;
        *self = Self::default();
        self.next_id = next_id;
        self.floor = floor;
        self.last_applied = last_applied;
        self.flying = flying;
        self.queued = queued;
        self.last_commit = last_commit;
        self.release = release;
    }

    pub fn offer(&mut self, tick: WheelTick, page: u32, pages: u32) -> WheelMove {
        if self.ignore {
            if lifting(&tick) {
                self.release_finger();
            }
            return if lifting(&tick) {
                WheelMove::Pass
            } else {
                WheelMove::Swallow
            };
        }
        if tick.momentum {
            if !self.live {
                return WheelMove::Swallow;
            }
            return self.settle(false, page, pages);
        }
        match tick.finger {
            Finger::Up => self.settle(false, page, pages),
            Finger::Cancelled => self.settle(true, page, pages),
            Finger::Down => self.track(tick, page, pages),
            Finger::None => self.notch(tick, page, pages),
        }
    }

    fn track(&mut self, tick: WheelTick, page: u32, pages: u32) -> WheelMove {
        if !self.live && tick.dx.abs() <= tick.dy.abs() {
            return WheelMove::Pass;
        }
        self.live = true;
        self.notch = 0.0;
        self.travel += tick.dx;
        WheelMove::Follow(shown_travel(self.travel, page, pages))
    }

    fn settle(&mut self, cancelled: bool, page: u32, pages: u32) -> WheelMove {
        if !self.live {
            return WheelMove::Pass;
        }
        let travel = self.travel;
        self.travel = 0.0;
        self.notch = 0.0;
        self.live = false;
        self.ignore = false;
        let effect = if cancelled || travel.abs() < SWIPE_POINTS {
            WheelMove::Cancel
        } else {
            let delta = if travel > 0.0 { 1 } else { -1 };
            if end_blocks(delta, page, pages) {
                WheelMove::Ease(delta)
            } else {
                WheelMove::Commit(delta)
            }
        };
        self.hand_glide(&effect);
        effect
    }

    fn notch(&mut self, tick: WheelTick, page: u32, pages: u32) -> WheelMove {
        if self.live || tick.dx.abs() <= tick.dy.abs() {
            return WheelMove::Pass;
        }
        self.notch += tick.dx;
        if self.notch.abs() < SWIPE_POINTS {
            return WheelMove::Hold;
        }
        let delta = if self.notch > 0.0 { 1 } else { -1 };
        self.notch = 0.0;
        let effect = if end_blocks(delta, page, pages) {
            WheelMove::Cancel
        } else {
            WheelMove::Commit(delta)
        };
        self.hand_glide(&effect);
        effect
    }

    /// A commit while one glide is flying replaces the single queued token.
    /// The page posts that token only if it kept the slot. A refused answer
    /// removes it. Nothing here remembers commits that have already posted.
    fn hand_glide(&mut self, effect: &WheelMove) {
        if matches!(effect, WheelMove::Commit(_)) {
            self.next_id = self.next_id.saturating_add(1);
            let id = self.next_id;
            if self.flying.is_none() {
                self.flying = Some(id);
            } else {
                self.queued = Some(id);
            }
            self.last_commit = Some(id);
        }
        self.release = true;
    }
}

/// The page the message handler stores. A rejected report leaves it unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct PageView {
    pub page: Option<u32>,
    pub pages: Option<u32>,
    pub sentence: Option<u32>,
    pub pin_missed: bool,
    pub pinned: Option<u32>,
}

impl Default for PageView {
    fn default() -> Self {
        Self {
            page: None,
            pages: None,
            sentence: None,
            pin_missed: false,
            pinned: None,
        }
    }
}

/// The script-message entry. `token` is the commit post. None is an ordinary
/// report. A token that was refused or retired does not change `view`.
pub fn deliver_page(
    swipe: &mut Swipe,
    view: &mut PageView,
    token: Option<u64>,
    page: u32,
    pages: u32,
    sentence: Option<u32>,
    missed: Option<bool>,
    pinned: Option<u32>,
) -> bool {
    if let Some(token) = token {
        if !swipe.take_commit_post(token) {
            return false;
        }
    }
    view.page = Some(page);
    view.pages = Some(pages);
    view.sentence = sentence;
    view.pin_missed = missed.unwrap_or(false);
    if let Some(pinned) = pinned {
        view.pinned = Some(pinned);
    }
    true
}

/// The one EPUB place. The gesture, the saved row, and the unfinished resume
/// live here. The inbox does not.
#[derive(Clone, Debug)]
pub struct Reader {
    pub swipe: Swipe,
    pub place: ReadingPlace,
    pub jump_href: String,
    pub jump_offset: Option<u32>,
    pub awaiting: Option<u32>,
    pub pending: u32,
    pub back: Option<ReadingPlace>,
    pub hold: Option<u32>,
    /// One reveal `poll_web` may run. A later poll does not invent another.
    pub reveal: Option<u32>,
    /// The one column walk started by a page-only resume. It ends when the
    /// shown column reaches it, or when a reader turn is saved. A resize does
    /// not end it.
    pub walk: Option<u32>,
    /// The caret character for this chapter. `None` until a caret hits,
    /// including a hit on character 0. A miss is not stored as 0.
    pub measured: Option<u32>,
    /// An img or svg identity for this document. It is not a character and
    /// it is not written.
    pub figure: Option<String>,
    /// Samples of the saved page that found no anchor. Two means a later
    /// size does not measure that page.
    pub sample_misses: u8,
    /// Lookups of `figure` that did not find the element. Two drops it.
    pub figure_misses: u8,
    /// A caret read is in flight. The poll does not chase and does not end
    /// the walk until the hit or the miss is applied.
    pub anchoring: bool,
    /// The sample this resize asked for. A post from an older sample is not
    /// the current character.
    pub probe: u64,
    /// The saved page number is not a chase. The frame may still move.
    /// A lost sample does not measure that page again.
    pub hold_layout: bool,
    /// The next restore answers a user post this reader refused or accepted.
    /// The page blocks an older restore until that answer arrives.
    pub restore_ack: bool,
    /// The loaded chapter document. A restore or a drop from an older document
    /// does not run. A user key does not carry this.
    pub restore_load: u64,
    /// The document `claim_document` last counted for the chapter on screen.
    /// Zero is a reader that has not loaded one, so a message with no document
    /// still applies. A loaded chapter rejects every other document.
    pub doc: u64,
    /// The fragment a cross-chapter link still has to measure in the target
    /// document. It is not a place and it is not written.
    pub fragment: Option<String>,
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            swipe: Swipe::default(),
            place: ReadingPlace {
                href: String::new(),
                page: 1,
                offset: 0,
            },
            jump_href: String::new(),
            jump_offset: None,
            awaiting: None,
            pending: 0,
            back: None,
            hold: None,
            reveal: None,
            walk: None,
            measured: None,
            figure: None,
            sample_misses: 0,
            figure_misses: 0,
            anchoring: false,
            probe: 0,
            hold_layout: false,
            restore_ack: false,
            restore_load: 0,
            doc: 0,
            fragment: None,
        }
    }
}

/// Who asked the page to turn. The script posts this source. Frames do not
/// choose it, and `web.turn` does not default every caller to a restore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnSource {
    /// Poll walking a stored page. `SPIKE_TURN` and `SPIKE_PLAY` are the host
    /// moving columns, so they use this too.
    Restore,
    /// A shell key, a shell edge, a page key, or a page edge.
    User,
}

/// The eval `web.turn` runs. `target` is the walk goal. `ack` answers the
/// user post the page already sent. Tests send this string.
pub fn turn_command(delta: i32, source: TurnSource, target: u32, ack: bool, load: u64) -> String {
    let source = match source {
        TurnSource::Restore => "restore",
        TurnSource::User => "user",
    };
    let ack = if ack { 1 } else { 0 };
    format!(
        "(function(){{var answer=(window.spikeTurn&&window.spikeTurn({delta},\"{source}\",{target},{ack},{load}))||\"no\";return JSON.stringify({{answer:answer}});}})()"
    )
}

/// The page started this turn. `no` did not, so the poll latch clears and the
/// walk stays where it was.
pub fn turn_accepted(answer: &str) -> bool {
    matches!(answer, "ok" | "queued")
}

/// The eval `drop_swipe` runs for a chapter, a resize, or a reopen.
pub fn drop_command(load: u64) -> String {
    format!("window.spikeFollowDrop&&window.spikeFollowDrop({load})")
}

/// The eval `release_swipe` runs. The page's answer is in the returned JSON.
pub fn release_command(delta: i32, token: u64) -> String {
    format!(
        "(function(){{var answer=(window.spikeFollowEnd&&window.spikeFollowEnd({delta},{token}))||\"no\";return JSON.stringify({{answer:answer,swipe:{token}}});}})()"
    )
}

/// Where one script message goes. A selection names its column, so `page` on
/// that message is not a column report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptRoute {
    Breaks,
    Link,
    Selection,
    Place,
}

pub fn script_route(value: &serde_json::Value) -> ScriptRoute {
    if value["kind"] == "breaks" {
        ScriptRoute::Breaks
    } else if value["kind"] == "link" {
        ScriptRoute::Link
    } else if value["kind"] == "selection" {
        ScriptRoute::Selection
    } else if value.get("page").is_some()
        || value.get("swipe").is_some()
        || value.get("frames").is_some()
        || value["kind"] == "turn"
    {
        ScriptRoute::Place
    } else {
        ScriptRoute::Selection
    }
}

/// The script message, parsed once. Absent fields stay absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageReport {
    pub dropped: bool,
    pub token: Option<u64>,
    pub kind: String,
    pub frames: bool,
    pub page: Option<u32>,
    pub pages: Option<u32>,
    pub sentence: Option<u32>,
    pub pinned: Option<u32>,
    pub missed: Option<bool>,
    pub hold: Option<u32>,
    /// The hold is a caret hit. Absent on a miss, including a miss that
    /// would otherwise look like character 0.
    pub measured: bool,
    /// The resize sample that produced this post. Zero when the page sent none.
    pub probe: u64,
    /// The poll's column walk. The same slide as a key, without a reader.
    pub chase: bool,
}

pub fn page_report(value: &serde_json::Value) -> PageReport {
    PageReport {
        dropped: value["dropped"].as_bool() == Some(true),
        token: value["swipe"].as_u64().filter(|token| *token > 0),
        kind: value["kind"].as_str().unwrap_or("").to_string(),
        frames: value.get("frames").is_some(),
        page: value["page"].as_u64().map(|page| page as u32),
        pages: value["pages"].as_u64().map(|pages| pages as u32),
        sentence: value["sentence"].as_u64().map(|offset| offset as u32),
        pinned: value["pinned"].as_u64().map(|offset| offset as u32),
        missed: value["missed"].as_bool(),
        hold: value["hold"].as_u64().map(|offset| offset as u32),
        measured: value["measured"].as_bool() == Some(true),
        probe: value["probe"].as_u64().unwrap_or(0),
        chase: value["chase"].as_bool() == Some(true),
    }
}

/// What the host does after the place is committed. `paint` is the footer.
/// A rejected or dropped message does not paint.
#[derive(Clone, Debug)]
pub struct Apply {
    pub accepted: bool,
    pub paint: bool,
    pub wrote: bool,
    pub reveal: Option<u32>,
    pub go: Option<ChapterMove>,
}

fn apply_idle(accepted: bool) -> Apply {
    Apply {
        accepted,
        paint: false,
        wrote: false,
        reveal: None,
        go: None,
    }
}

/// The settings file. An empty book or page 0 is not a successful write:
/// `record_progress` returns `Ok` and stores nothing in that case.
pub trait PlaceFile {
    fn write_place(&mut self, place: &ReadingPlace) -> bool;
    fn resume(&self) -> ReadingPlace;
    /// A caret hit, including character 0. The default writes the page row
    /// and records no caret, which is the old page-only file.
    fn write_anchor(&mut self, place: &ReadingPlace) -> bool {
        self.write_place(place)
    }
    fn resume_caret(&self) -> bool {
        false
    }
}

/// A swipe token or a key or edge (`kind: "turn"`). A column walk posts
/// `frames` too, and those frames are the slide timing, not a reader.
fn user_turn(report: &PageReport) -> bool {
    if report.token.is_some() {
        return true;
    }
    !report.chase && report.kind == "turn"
}

/// What one script message does to the column element.
/// `None` means leave the mailbox. A user turn or a character hold clears it.
/// A figure is kept when its probe is current, or when the post names the
/// element already kept and sends no probe. A late probe is left alone.
/// A figure with no page is not a place message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FigureMail {
    pub id: Option<String>,
    pub lost: bool,
}

pub fn figure_mail(
    value: &serde_json::Value,
    probe: u64,
    known: Option<&str>,
) -> Option<FigureMail> {
    let report = page_report(value);
    if user_turn(&report) || report.hold.is_some() {
        return Some(FigureMail {
            id: None,
            lost: false,
        });
    }
    let Some(id) = value
        .get("figure")
        .and_then(|item| item.as_str())
        .filter(|id| !id.is_empty())
    else {
        return None;
    };
    if report.page.is_none() {
        return None;
    }
    let generation = value.get("probe").and_then(|item| item.as_u64());
    let current = generation.is_some_and(|sent| sent == probe);
    let same = generation.is_none() && known == Some(id);
    if !(current || same) {
        return None;
    }
    Some(FigureMail {
        id: Some(id.to_string()),
        lost: report.missed == Some(true),
    })
}

pub fn keep_message_figure(
    figure: &mut Option<String>,
    lost: &mut bool,
    value: &serde_json::Value,
    probe: u64,
    known: Option<&str>,
) {
    if let Some(mail) = figure_mail(value, probe, known) {
        *figure = mail.id;
        *lost = mail.lost;
    }
}

pub fn drop_figure_mail(figure: &mut Option<String>, lost: &mut bool) {
    *figure = None;
    *lost = false;
}

pub fn remember_figure(reader: &mut Reader, id: Option<String>) {
    if let Some(id) = id {
        reader.figure = Some(id);
    }
}

pub fn figure_flags(id: &Option<String>, lost: bool) -> (bool, bool) {
    (id.is_some() && !lost, lost)
}

/// A missed lookup is not a new element. Noting it would put the id back
/// after the frame had dropped it.
pub fn found_figure(id: Option<String>, lost: bool) -> Option<String> {
    if lost {
        None
    } else {
        id
    }
}

/// The loss was applied. A resting frame has not read it yet.
pub fn spent_figure_loss(lost: bool, draw: Draw) -> bool {
    lost && !matches!(draw, Draw::Rest)
}

/// The find was applied. A later frame should not read it again.
pub fn spent_figure_find(found: bool, draw: Draw) -> bool {
    found && !matches!(draw, Draw::Rest)
}

fn clear_resume(reader: &mut Reader) {
    reader.jump_href.clear();
    reader.jump_offset = None;
    reader.awaiting = None;
    reader.pending = 0;
    reader.hold = None;
    reader.reveal = None;
    reader.walk = None;
    reader.measured = None;
    reader.figure = None;
    reader.sample_misses = 0;
    reader.figure_misses = 0;
    reader.anchoring = false;
    reader.hold_layout = false;
    reader.fragment = None;
}

/// A place message belongs to the chapter now loaded.
/// A reader that has not loaded a document still accepts one, which is how
/// the existing tests post a page. A loaded document accepts only its own count.
pub fn message_is_this_document(live: u64, posted: u64) -> bool {
    live == 0 || posted == live
}

/// A place report names the restore epoch it was posted under.
/// A message with no `load` is accepted. `restore_load` 0 accepts any `load`.
/// The page script always sends `load`, and opening a book claims a document
/// before that page posts, so a loaded reader is not 0. A loaded reader
/// rejects an older epoch. The epoch just given to the page still applies.
pub fn report_is_this_epoch(live: u64, value: &serde_json::Value) -> bool {
    let Some(posted) = value.get("load").and_then(|load| load.as_u64()) else {
        return true;
    };
    live == 0 || posted == live
}

/// A choice that stays on the open document. Reports already posted keep the
/// epoch they were issued under. The host tells the page this epoch next.
pub fn advance_open_document(reader: &mut Reader) -> u64 {
    reset_restore_epoch(reader);
    reader.restore_load
}

/// Why a sentence on the open document is being shown.
/// Contents, a link, a bookmark, and a find hit are one choice.
/// A resize or a restore keeps the epoch, so that reflow does not cancel the place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SentenceCause {
    Chosen,
    Held,
}

/// The sentence to show, and the epoch a choice retired the previous reports under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenSentence {
    pub offset: u32,
    pub load: Option<u64>,
}

/// The one decision for a sentence inside the document already open.
/// A choice advances the restore epoch. A held reveal does not.
pub fn sentence_in_open_document(
    reader: &mut Reader,
    offset: u32,
    cause: SentenceCause,
) -> OpenSentence {
    let load = match cause {
        SentenceCause::Chosen => Some(advance_open_document(reader)),
        SentenceCause::Held => None,
    };
    OpenSentence { offset, load }
}

/// A contents or place choice on a PDF. The glide and the key slide are
/// dropped, and a page choice is written through the same place file as a turn.
pub fn apply_contents_choice<G, M>(
    glide: &mut Option<G>,
    motion: &mut Option<M>,
    drag: &mut f64,
    saved_page: &mut u32,
    saved_offset: &mut u32,
    go: &ChapterMove,
    store: &mut dyn PlaceFile,
) -> bool {
    if !contents_choice_retires_pdf_motion(go) {
        return false;
    }
    *glide = None;
    *motion = None;
    *drag = 0.0;
    let ChapterMove::Page { page } = *go else {
        return false;
    };
    let Some((page, offset)) = landed_progress(*saved_page, *saved_offset, page) else {
        return false;
    };
    let wrote = store.write_place(&ReadingPlace {
        href: String::new(),
        page,
        offset,
    });
    if wrote {
        *saved_page = page;
        *saved_offset = offset;
    }
    wrote
}

/// The handler reports only while its view is the one on screen.
/// Dismissing it is what stops the previous book. Both books count documents
/// from zero, so the document number alone is not a book.
pub fn accept_place_message(reports: bool, live_doc: u64, posted_doc: u64) -> bool {
    reports && message_is_this_document(live_doc, posted_doc)
}

/// An accepted contents choice is the place. The unfinished PDF slide does not
/// land after it and write a page the choice already replaced.
pub fn contents_choice_retires_pdf_motion(go: &ChapterMove) -> bool {
    matches!(
        go,
        ChapterMove::Stay
            | ChapterMove::Reveal { .. }
            | ChapterMove::Chapter { .. }
            | ChapterMove::Page { .. }
    )
}

/// Accept a script message. The place changes only after `write_place`
/// returns true. A refused token, including one past the floor, changes nothing.
pub fn handle_page(
    reader: &mut Reader,
    value: &serde_json::Value,
    href: &str,
    store: &mut dyn PlaceFile,
) -> Apply {
    let posted_doc = value["doc"].as_u64().unwrap_or(0);
    if !message_is_this_document(reader.doc, posted_doc) {
        return apply_idle(false);
    }
    if !report_is_this_epoch(reader.restore_load, value) {
        return apply_idle(false);
    }
    let report = page_report(value);
    if report.dropped {
        if let Some(token) = report.token {
            reader.swipe.retire(token);
        }
        return Apply {
            accepted: true,
            paint: false,
            wrote: false,
            reveal: None,
            go: None,
        };
    }
    if reader.fragment.is_some() || foreign_document(reader, href) {
        return apply_idle(true);
    }
    if report.hold.is_some()
        && report.token.is_none()
        && !report.frames
        && report.kind != "turn"
        && report.pinned.is_none()
    {
        if report.measured && report.probe == reader.probe {
            reader.measured = report.hold;
        }
        return Apply {
            accepted: true,
            paint: true,
            wrote: false,
            reveal: None,
            go: None,
        };
    }
    if value.get("figure").is_some()
        && report.hold.is_none()
        && report.pinned.is_none()
        && report.kind != "turn"
    {
        return apply_idle(true);
    }
    if report.missed == Some(true)
        && report.token.is_none()
        && report.kind != "turn"
        && report.hold.is_none()
        && report.pinned.is_none()
    {
        if report.probe == reader.probe {
            reader.anchoring = false;
            reader.hold_layout = true;
        }
        return Apply {
            accepted: true,
            paint: false,
            wrote: false,
            reveal: None,
            go: None,
        };
    }
    if report.missed == Some(true)
        && report.token.is_none()
        && report.kind != "turn"
        && report.hold.is_none()
        && report.pinned.is_some()
        && reader.anchoring
    {
        reader.anchoring = false;
        reader.hold_layout = true;
        return Apply {
            accepted: true,
            paint: false,
            wrote: false,
            reveal: None,
            go: None,
        };
    }
    let landed = report.missed != Some(true)
        && report.pinned.is_some()
        && report.token.is_none()
        && report.kind != "turn"
        && (report.kind == "anchor" || reader.anchoring);
    if landed {
        let Some(page) = report.page.filter(|page| *page > 0) else {
            return apply_idle(true);
        };
        let offset = report.pinned.unwrap_or(0);
        let place = ReadingPlace {
            href: reader.place.href.clone(),
            page,
            offset,
        };
        let changed = place != reader.place;
        if !store.write_anchor(&place) {
            reader.restore_ack = true;
            reader.anchoring = false;
            reader.hold_layout = true;
            reader.measured = Some(offset);
            return apply_idle(false);
        }
        reader.place = place;
        reader.measured = Some(offset);
        reader.walk = None;
        reader.anchoring = false;
        reader.hold_layout = false;
        reader.restore_ack = true;
        return Apply {
            accepted: true,
            paint: true,
            wrote: changed,
            reveal: None,
            go: None,
        };
    }
    if let Some(token) = report.token {
        if !reader.swipe.holds(token) {
            reader.restore_ack = true;
            return apply_idle(false);
        }
    }
    if user_turn(&report) {
        let mut next = reader.clone();
        if let Some(token) = report.token {
            if !next.swipe.take_commit_post(token) {
                reader.restore_ack = true;
                return apply_idle(false);
            }
        }
        let shown = report.page.unwrap_or(reader.place.page.max(1));
        let sentence = report.sentence.unwrap_or(0);
        let (page, offset) = next_sentence(
            reader.place.page.max(1),
            reader.place.offset,
            shown,
            sentence,
        );
        let place = ReadingPlace {
            href: reader.place.href.clone(),
            page,
            offset,
        };
        let changed = place != reader.place;
        if changed && !store.write_place(&place) {
            reader.restore_ack = true;
            return apply_idle(false);
        }
        next.place = place;
        clear_resume(&mut next);
        next.restore_ack = true;
        *reader = next;
        return Apply {
            accepted: true,
            paint: true,
            wrote: changed,
            reveal: None,
            go: None,
        };
    }
    let shown = report.page.unwrap_or(reader.place.page.max(1));
    let polled = poll_chapter(ChapterPoll {
        saved_href: &reader.place.href,
        saved_page: reader.place.page,
        saved_offset: reader.place.offset,
        jump_href: &reader.jump_href,
        jump_offset: reader.jump_offset,
        awaiting: reader.awaiting,
        pending: reader.pending,
        hold: reader.hold,
        shown,
        href,
        ready: true,
        reported: report.page.is_some(),
        pinned: report.pinned,
        missed: report.missed == Some(true),
        sentence: report.sentence.unwrap_or(0),
        still_turning: false,
    });
    let pin_match = reader.awaiting.is_some()
        && report.pinned == reader.awaiting
        && report.missed != Some(true);
    let pin_message = report.pinned.is_some() || report.missed.is_some();
    let write = polled.record
        && if pin_message {
            polled.pin_write && pin_match
        } else {
            changed_chapter(&reader.place.href, href)
        };
    let place = ReadingPlace {
        href: polled.saved_href,
        page: polled.saved_page,
        offset: polled.saved_offset,
    };
    if write && !store.write_place(&place) {
        return apply_idle(false);
    }
    let reveal = polled.reveal_jump.or(polled.reveal_pending);
    reader.jump_offset = polled.jump_offset;
    reader.awaiting = polled.awaiting;
    reader.pending = polled.pending;
    reader.hold = polled.hold;
    reader.reveal = reveal;
    if write {
        reader.place = place;
    }
    if !layout_held(reader) && reader.walk.is_some_and(|target| shown >= target) {
        reader.walk = None;
    }
    Apply {
        accepted: true,
        paint: true,
        wrote: write,
        reveal,
        go: None,
    }
}

/// Chapter and return. The saved place is already the last accepted turn.
/// The href on screen can be a chapter that has not been written. That href
/// is not the return. This does not read the inbox.
pub fn handle_chapter(
    reader: &mut Reader,
    pick: isize,
    marks: &[ChapterMark],
    spine: &[String],
    href: &str,
    store: &mut dyn PlaceFile,
) -> Apply {
    if reader.fragment.take().is_some() {
        reader.jump_href.clear();
        reader.jump_offset = None;
    }
    let nav = navigate(NavIn {
        shown_href: href,
        shown_page: reader.place.page.max(1),
        saved_href: &reader.place.href,
        saved_page: reader.place.page.max(1),
        saved_offset: reader.place.offset,
        jump_href: &reader.jump_href,
        jump_offset: reader.jump_offset,
        awaiting: reader.awaiting,
        pending: reader.pending,
        back: reader.back.clone(),
        marks,
        spine,
        pick,
    });
    let place = ReadingPlace {
        href: nav.saved_href.clone(),
        page: nav.saved_page,
        offset: nav.saved_offset,
    };
    if nav.record && !store.write_place(&place) {
        return apply_idle(false);
    }
    reader.place = place;
    reader.back = nav.back;
    reader.pending = nav.pending;
    reader.jump_href = nav.jump_href;
    reader.jump_offset = nav.jump_offset;
    reader.awaiting = nav.awaiting;
    reader.reveal = None;
    if !matches!(nav.go, ChapterMove::Stay) {
        reader.walk = None;
        reader.measured = None;
        reader.figure = None;
        reader.sample_misses = 0;
        reader.figure_misses = 0;
        reader.anchoring = false;
        reader.hold_layout = false;
    }
    let reveal = match nav.go {
        ChapterMove::Reveal { offset } => Some(offset),
        _ => None,
    };
    Apply {
        accepted: true,
        paint: false,
        wrote: nav.record,
        reveal,
        go: Some(nav.go),
    }
}

/// A same-book href. `href` is the spine item. `fragment` has no `#`.
/// `same` means that item is the document on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkDest {
    pub href: String,
    pub fragment: String,
    pub same: bool,
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return None;
        }
        let hi = hex_digit(bytes[index + 1])?;
        let lo = hex_digit(bytes[index + 2])?;
        let byte = (hi << 4) | lo;
        if byte == 0 {
            return None;
        }
        out.push(byte);
        index += 3;
    }
    String::from_utf8(out).ok()
}

fn link_scheme(raw: &str) -> bool {
    if raw.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = raw.split_once(':') else {
        return false;
    };
    let scheme = scheme.trim();
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-')
}

pub(crate) fn spine_key(href: &str) -> String {
    let owned = percent_decode(href).unwrap_or_else(|| href.to_string());
    let href = owned.as_str();
    href.split(['#', '?'])
        .next()
        .unwrap_or(href)
        .trim_start_matches('/')
        .to_string()
}

fn clean_link_path(path: &str) -> Option<String> {
    let mut out = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if out.pop().is_none() {
                return None;
            }
            continue;
        }
        if part.contains(':') {
            return None;
        }
        out.push(part);
    }
    if out.is_empty() {
        return None;
    }
    Some(format!("/{}", out.join("/")))
}

fn join_link(shown: &str, rel: &str) -> Option<String> {
    let rel = rel.trim();
    if rel.is_empty() {
        return None;
    }
    let combined = if rel.starts_with('/') {
        rel.to_string()
    } else {
        let base = shown.split(['#', '?']).next().unwrap_or(shown);
        let dir = match base.rfind('/') {
            Some(index) => &base[..=index],
            None => "/",
        };
        format!("{dir}{rel}")
    };
    clean_link_path(&combined)
}

/// Resolve one clicked href against the open spine. A fragment-only href stays
/// on `shown`. `#` is not a spine item. An external scheme, a path that leaves
/// the book, or an unknown spine item is `None`.
pub fn resolve_link(raw: &str, shown: &str, spine: &[String]) -> Option<LinkDest> {
    let decoded = percent_decode(raw.trim())?;
    let raw = decoded.as_str();
    if raw.is_empty() || link_scheme(raw) || raw.contains('\\') || raw.contains('?') {
        return None;
    }
    let (path, fragment) = match raw.split_once('#') {
        Some((path, fragment)) => (path.trim(), fragment.trim()),
        None => (raw, ""),
    };
    if fragment.contains('#') || fragment.contains('/') {
        return None;
    }
    let href = if path.is_empty() {
        if fragment.is_empty() {
            return None;
        }
        shown.to_string()
    } else {
        let joined = join_link(shown, path)?;
        let matched = spine
            .iter()
            .find(|item| spine_key(item) == spine_key(&joined))?;
        matched.clone()
    };
    if !spine.iter().any(|item| spine_key(item) == spine_key(&href)) {
        return None;
    }
    let same = spine_key(&href) == spine_key(shown);
    Some(LinkDest {
        href,
        fragment: fragment.to_string(),
        same,
    })
}

fn spine_pos(spine: &[String], href: &str) -> Option<usize> {
    spine
        .iter()
        .position(|item| spine_key(item) == spine_key(href))
}

fn link_mark(href: &str, offset: u32) -> ChapterMark {
    ChapterMark {
        title: String::new(),
        href: href.to_string(),
        offset,
        page: 0,
    }
}

fn link_message<'a>(value: &'a serde_json::Value) -> (&'a str, u64, bool, Option<u32>) {
    (
        value["raw"].as_str().unwrap_or(""),
        value["load"].as_u64().unwrap_or(u64::MAX),
        value["measured"].as_bool() == Some(true),
        value["offset"].as_u64().map(|offset| offset as u32),
    )
}

fn link_ready(dest: &LinkDest, measured: bool, offset: Option<u32>) -> bool {
    dest.fragment.is_empty() || !dest.same || (measured && offset.is_some())
}

fn foreign_document(reader: &Reader, href: &str) -> bool {
    if href.is_empty() || reader.place.href.is_empty() {
        return false;
    }
    let shown = spine_key(href);
    shown != spine_key(&reader.place.href) && shown != spine_key(&reader.jump_href)
}

fn link_matches_pending(reader: &Reader, raw: &str, shown: &str, spine: &[String]) -> bool {
    let Some(pending) = reader.fragment.as_deref() else {
        return false;
    };
    let Some(dest) = resolve_link(raw, shown, spine) else {
        return false;
    };
    dest.fragment == pending && spine_key(&dest.href) == spine_key(&reader.jump_href)
}

/// One `kind: "link"` message. The page measured `offset` with the same
/// character walk as a caret. A cross-chapter fragment carries no offset
/// until the target document measures it. The return place stays on the
/// reader. The file receives the destination sentence only.
/// While that measurement is still waiting, a page message is not a turn
/// and does not write. A click retires it only when the click can move.
/// `auto` is that measurement. A document that is neither the stored
/// sentence nor the jump does not write.
pub fn handle_link_message(
    reader: &mut Reader,
    value: &serde_json::Value,
    shown: &str,
    spine: &[String],
    store: &mut dyn PlaceFile,
) -> Apply {
    let (raw, load, measured, offset) = link_message(value);
    if load != reader.restore_load {
        return apply_idle(false);
    }
    let auto = value["auto"].as_bool() == Some(true);
    if reader.fragment.is_some() {
        if link_matches_pending(reader, raw, shown, spine) {
            return finish_link(reader, raw, measured, offset, shown, spine, store);
        }
        let ready =
            resolve_link(raw, shown, spine).is_some_and(|dest| link_ready(&dest, measured, offset));
        if auto || !ready {
            return apply_idle(false);
        }
        reader.fragment = None;
        reader.jump_href.clear();
        reader.jump_offset = None;
    } else if auto {
        return apply_idle(false);
    }
    let Some(dest) = resolve_link(raw, shown, spine) else {
        return apply_idle(false);
    };
    if !link_ready(&dest, measured, offset) {
        return apply_idle(false);
    }
    let _ = reader.swipe.cancel_release();
    if !dest.fragment.is_empty() && !dest.same {
        reader.fragment = Some(dest.fragment);
        reader.jump_href = dest.href.clone();
        reader.jump_offset = None;
        let Some(index) = spine_pos(spine, &dest.href) else {
            reader.fragment = None;
            reader.jump_href.clear();
            return apply_idle(false);
        };
        return Apply {
            accepted: true,
            paint: false,
            wrote: false,
            reveal: None,
            go: Some(ChapterMove::Chapter { index, offset: 0 }),
        };
    }
    let offset = if dest.fragment.is_empty() {
        0
    } else {
        offset.unwrap_or(0)
    };
    let href = if dest.same {
        shown.to_string()
    } else {
        dest.href
    };
    let mark = link_mark(&href, offset);
    let mut applied = handle_chapter(reader, 0, &[mark], spine, shown, store);
    if matches!(applied.go, Some(ChapterMove::Stay)) {
        return applied;
    }
    reader.fragment = None;
    reveal_shown(reader, &mut applied, &href, shown);
    applied
}

fn reveal_shown(reader: &mut Reader, applied: &mut Apply, target: &str, shown: &str) {
    let Some(ChapterMove::Chapter { offset, .. }) = applied.go else {
        return;
    };
    if spine_key(target) != spine_key(shown) {
        return;
    }
    reader.jump_offset = None;
    reader.awaiting = Some(offset);
    applied.reveal = Some(offset);
    applied.go = Some(ChapterMove::Reveal { offset });
}

fn finish_link(
    reader: &mut Reader,
    raw: &str,
    measured: bool,
    offset: Option<u32>,
    shown: &str,
    spine: &[String],
    store: &mut dyn PlaceFile,
) -> Apply {
    let pending = reader.fragment.clone().unwrap_or_default();
    let Some(dest) = resolve_link(raw, shown, spine) else {
        return apply_idle(false);
    };
    if dest.fragment != pending || spine_key(&dest.href) != spine_key(&reader.jump_href) {
        return apply_idle(false);
    }
    reader.fragment = None;
    if !measured {
        let origin = reader.place.href.clone();
        let origin_offset = reader.place.offset;
        reader.jump_href = origin.clone();
        reader.jump_offset = Some(origin_offset);
        let Some(index) = spine_pos(spine, &origin) else {
            reader.jump_href.clear();
            reader.jump_offset = None;
            return apply_idle(false);
        };
        return Apply {
            accepted: true,
            paint: false,
            wrote: false,
            reveal: None,
            go: Some(ChapterMove::Chapter {
                index,
                offset: origin_offset,
            }),
        };
    }
    let Some(offset) = offset else {
        reader.fragment = Some(pending);
        return apply_idle(false);
    };
    let target = dest.href.clone();
    let mark = link_mark(&target, offset);
    let origin = reader.place.href.clone();
    let mut applied = handle_chapter(reader, 0, &[mark], spine, &origin, store);
    if !applied.accepted {
        return applied;
    }
    reveal_shown(reader, &mut applied, &target, shown);
    reader.fragment = None;
    applied
}

/// Cancel a gesture that has not posted. The accepted place stays on disk.
pub fn leave(reader: &mut Reader) {
    let _ = reader.swipe.cancel_release();
}

/// The resume row becomes the place. An offset above 0 waits as `pending`.
/// `caret` is a saved hit, including character 0. An old row has no caret
/// and a zero offset, and that row still walks. Nothing is written.
pub fn open_place(reader: &mut Reader, page: u32, offset: u32, href: &str, caret: bool) {
    let page = page.max(1);
    reader.place = ReadingPlace {
        href: href.to_string(),
        page,
        offset,
    };
    reader.pending = if offset > 0 { offset } else { 0 };
    reader.jump_href.clear();
    reader.jump_offset = None;
    reader.awaiting = None;
    reader.hold = None;
    reader.back = None;
    reader.reveal = None;
    reader.measured = if caret || offset > 0 {
        Some(offset)
    } else {
        None
    };
    reader.figure = None;
    reader.sample_misses = 0;
    reader.figure_misses = 0;
    reader.anchoring = false;
    reader.hold_layout = false;
    reader.fragment = None;
    reader.walk = if caret {
        None
    } else {
        resume_walk(page, offset)
    };
}

/// An explicit stop ends the walk. A shell key does not call this before
/// its post is saved. A resize does not. The saved row is not written here.
pub fn stop_walk(reader: &mut Reader) {
    reader.walk = None;
}

/// The page dropped its gesture, or a new chapter document replaced it.
/// Empty swipe slots still count: a restore slide is not one of those slots.
pub fn retire_restore(reader: &mut Reader) -> bool {
    stop_for_ease(&mut reader.swipe)
}

pub fn stop_for_ease(swipe: &mut Swipe) -> bool {
    swipe.cancel_release()
}

pub fn accept_landed(swipe: &mut Swipe, token: u64) -> bool {
    token == 0 || swipe.take_commit_post(token)
}

fn lifting(tick: &WheelTick) -> bool {
    matches!(tick.finger, Finger::Up | Finger::Cancelled) && !tick.momentum
}

pub fn offer_while_reading(
    swipe: &mut Swipe,
    tick: WheelTick,
    page: u32,
    pages: u32,
    trip: crate::ease::Trip,
) -> WheelMove {
    if !crate::ease::pointer_turns(trip) {
        if lifting(&tick) && (swipe.ignore || swipe.live) {
            swipe.release_finger();
        }
        WheelMove::Swallow
    } else {
        swipe.offer(tick, page, pages)
    }
}

/// The chapter load just claimed `doc`. The restore epoch moves with it.
pub fn claim_loaded_document(reader: &mut Reader, doc: u64) {
    reset_restore_epoch(reader);
    reader.doc = doc;
}

pub fn reset_restore_epoch(reader: &mut Reader) {
    reader.restore_load = reader.restore_load.saturating_add(1);
    reader.restore_ack = false;
    reader.measured = None;
    reader.figure = None;
    reader.sample_misses = 0;
    reader.figure_misses = 0;
    reader.anchoring = false;
    reader.hold_layout = false;
    reader.probe = reader.probe.saturating_add(1);
}

/// `place_page` on a cancel. The drop stops an animation that has not posted.
/// The walk and a commit that already posted stay.
pub fn cancel_placed_page(reader: &mut Reader) -> String {
    drop_command(reader.restore_load)
}

pub fn open_resume(reader: &mut Reader, store: &dyn PlaceFile) {
    let resumed = store.resume();
    open_place(
        reader,
        resumed.page,
        resumed.offset,
        &resumed.href,
        store.resume_caret(),
    );
}

/// A stored page with no sentence is walked column by column. A sentence
/// waits for its reveal and does not walk. Page 1 is already showing.
pub fn resume_walk(page: u32, offset: u32) -> Option<u32> {
    let page = page.max(1);
    if offset > 0 || page <= 1 {
        None
    } else {
        Some(page)
    }
}

/// The poll calls the programmatic page turn while the shown column is
/// still short of `resume_walk`. A turn already in flight is not repeated.
pub fn resume_turn(target: Option<u32>, shown: u32, pages: u32, turning: bool) -> bool {
    target.is_some_and(|target| pages > 1 && shown < target && !turning)
}

/// One poll. Reaching the stored column ends the walk. The command is the
/// restore eval, or nothing when the walk is over or a turn is in flight.
pub struct ResumePoll {
    pub walk: Option<u32>,
    pub command: Option<String>,
}

pub fn poll_resume(
    walk: Option<u32>,
    shown: u32,
    pages: u32,
    turning: bool,
    ack: bool,
    load: u64,
) -> ResumePoll {
    let walk = match walk {
        Some(target) if shown >= target => None,
        other => other,
    };
    let command = if resume_turn(walk, shown, pages, turning) {
        let target = walk.unwrap_or(0);
        Some(turn_command(1, TurnSource::Restore, target, ack, load))
    } else {
        None
    };
    ResumePoll { walk, command }
}

/// The poll while a caret read is in flight, or the saved page number is not
/// a chase. A column past that page does not end the walk.
fn layout_held(reader: &Reader) -> bool {
    reader.anchoring || reader.measured.is_some() || reader.hold_layout
}

/// One draw, the same inputs `place_page` has. The test drives this for
/// several frames. A matching box does not cancel a gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTick {
    pub fitted_same: bool,
    pub zoom_same: bool,
    pub stage_same: bool,
    pub reflowable: bool,
    pub settled: bool,
    pub awaiting: bool,
    pub hold: Option<u32>,
    pub hold_probe: u64,
    pub missed: bool,
    pub timed_out: bool,
    pub shown: u32,
    pub pages: u32,
    pub pages_known: bool,
    /// The page found an img or svg in the sampled column.
    pub figure: bool,
    /// The page could not find the element it had been asked to align.
    pub figure_lost: bool,
}

impl FrameTick {
    pub fn new(
        fitted_same: bool,
        zoom_same: bool,
        stage_same: bool,
        reflowable: bool,
        settled: bool,
        awaiting: bool,
    ) -> Self {
        Self {
            fitted_same,
            zoom_same,
            stage_same,
            reflowable,
            settled,
            awaiting,
            hold: None,
            hold_probe: 0,
            missed: false,
            timed_out: false,
            shown: 1,
            pages: 1,
            pages_known: true,
            figure: false,
            figure_lost: false,
        }
    }
}

/// What this draw does to the web view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    /// Leave the view where it is.
    Rest,
    /// Ask the page for the caret. This draw drops a gesture painted for the
    /// old box. Later draws of the same wait do not.
    Measure { page: u32, probe: u64 },
    /// Move the view to the fitted box. `reveal` seeks that character after
    /// the move. None does not seek a page number.
    Move { reveal: Option<u32> },
    /// Move the view, then scroll the stored element into its column.
    Align,
}

/// `cancel` drops a gesture that has not posted. `pump` asks for another draw.
/// A walk that is not being chased does not pump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawPlan {
    pub draw: Draw,
    pub cancel: bool,
    pub pump: bool,
}

fn chase_due(reader: &Reader, tick: FrameTick) -> bool {
    if layout_held(reader) {
        return false;
    }
    if !tick.pages_known {
        return reader.walk.is_some();
    }
    resume_turn(reader.walk, tick.shown, tick.pages, false)
}

fn plan(draw: Draw, cancel: bool, pump: bool) -> DrawPlan {
    DrawPlan { draw, cancel, pump }
}

fn lost(reader: &Reader) -> bool {
    reader.sample_misses >= 2
}

fn reveal_char(reader: &mut Reader, offset: u32) -> DrawPlan {
    reader.anchoring = true;
    reader.hold_layout = false;
    reader.sample_misses = 0;
    reader.figure = None;
    reader.figure_misses = 0;
    plan(
        Draw::Move {
            reveal: Some(offset),
        },
        false,
        false,
    )
}

/// The draw `place_page` applies. A character reveal does not wait for the
/// chrome settle flag. A page number is measured only while that column's
/// layout is still the one on screen, and only twice. After that the box
/// moves and the page number is not sampled again.
pub fn layout_frame(reader: &mut Reader, tick: FrameTick) -> DrawPlan {
    // The first align has cleared the wait. A later miss and a later find
    // both arrive with awaiting false.
    if tick.figure_lost {
        reader.figure_misses = reader.figure_misses.saturating_add(1);
        reader.anchoring = false;
        reader.hold_layout = true;
        if reader.figure_misses >= 2 {
            reader.figure = None;
            reader.sample_misses = 2;
            return plan(Draw::Move { reveal: None }, false, false);
        }
        return plan(Draw::Align, false, true);
    }
    if tick.figure && reader.figure.is_some() {
        reader.sample_misses = 0;
        reader.figure_misses = 0;
        reader.anchoring = false;
        reader.hold_layout = true;
        return plan(Draw::Align, false, false);
    }
    if tick.awaiting {
        let hold = current_hold(tick.hold, tick.hold_probe, reader.probe);
        let missed = tick.missed && hold.is_none();
        let timed_out = tick.timed_out && hold.is_none();
        match finish_hold_offset(hold, missed, timed_out) {
            HoldFinish::Wait => return plan(Draw::Rest, false, true),
            HoldFinish::Timeout => {
                if reader.sample_misses == 0 {
                    reader.sample_misses = 1;
                    reader.probe = reader.probe.saturating_add(1);
                    reader.anchoring = true;
                    reader.hold_layout = true;
                    let page = reader.walk.unwrap_or(0);
                    let probe = reader.probe;
                    return plan(Draw::Measure { page, probe }, false, true);
                }
                reader.sample_misses = 2;
                reader.anchoring = false;
                reader.hold_layout = true;
                return plan(Draw::Move { reveal: None }, false, false);
            }
            HoldFinish::Miss => {
                reader.probe = reader.probe.saturating_add(1);
                reader.sample_misses = 2;
                reader.figure = None;
                reader.figure_misses = 0;
                reader.anchoring = false;
                reader.hold_layout = true;
                return plan(Draw::Move { reveal: None }, false, false);
            }
            HoldFinish::Reveal(offset) => return reveal_char(reader, offset),
        }
    }
    let chase = chase_due(reader, tick);
    match place_step(tick.fitted_same, tick.zoom_same, tick.stage_same) {
        PlaceStep::Hold => plan(Draw::Rest, false, chase),
        PlaceStep::Cancel { reflow: false } => plan(Draw::Rest, true, chase),
        PlaceStep::Cancel { reflow: true } => {
            if tick.reflowable {
                if let Some(offset) = reader.measured {
                    return reveal_char(reader, offset);
                }
                if reader.figure.is_some() {
                    reader.anchoring = false;
                    reader.hold_layout = true;
                    return plan(Draw::Align, false, false);
                }
                if lost(reader) {
                    reader.hold_layout = true;
                    return plan(Draw::Move { reveal: None }, false, false);
                }
                if tick.settled {
                    if reader.anchoring {
                        return plan(Draw::Rest, false, true);
                    }
                    reader.probe = reader.probe.saturating_add(1);
                    reader.anchoring = true;
                    let page = reader.walk.unwrap_or(0);
                    let probe = reader.probe;
                    return plan(Draw::Measure { page, probe }, true, true);
                }
            }
            plan(Draw::Move { reveal: None }, false, chase)
        }
    }
}

pub fn poll_anchored(reader: &mut Reader, shown: u32, pages: u32, turning: bool) -> ResumePoll {
    if layout_held(reader) {
        return ResumePoll {
            walk: reader.walk,
            command: None,
        };
    }
    let polled = poll_resume(
        reader.walk,
        shown,
        pages,
        turning,
        reader.restore_ack,
        reader.restore_load,
    );
    reader.walk = polled.walk;
    polled
}

pub fn hold_eval(page: u32, probe: u64) -> String {
    format!("window.spikeHold&&window.spikeHold({page},{probe})")
}

/// The offset `finish_hold` may reveal. A miss and a timeout are not character 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldFinish {
    Wait,
    Miss,
    Timeout,
    Reveal(u32),
}

pub fn finish_hold_offset(hold: Option<u32>, missed: bool, timed_out: bool) -> HoldFinish {
    if let Some(ch) = hold {
        return HoldFinish::Reveal(ch);
    }
    if missed {
        return HoldFinish::Miss;
    }
    if timed_out {
        return HoldFinish::Timeout;
    }
    HoldFinish::Wait
}

/// A hold already in the inbox is current only when it is the sample still
/// being asked for.
pub fn current_hold(hold: Option<u32>, hold_probe: u64, reader_probe: u64) -> Option<u32> {
    hold.filter(|_| hold_probe == reader_probe)
}

pub fn take_reveal(reader: &mut Reader) -> Option<u32> {
    reader.reveal.take()
}

/// A page report from the commit glide. `token` is the value the glide posted.
/// None means an ordinary report, including a reveal pin. A stale token does
/// not change the stored page.
pub fn swipe_progress(
    swipe: &mut Swipe,
    token: Option<u64>,
    page: u32,
    sentence: u32,
) -> Option<(u32, u32)> {
    match token {
        Some(token) => swipe.take_commit_post(token).then_some((page, sentence)),
        None => Some((page, sentence)),
    }
}

fn shown_travel(travel: f64, page: u32, pages: u32) -> f64 {
    if end_blocks(if travel > 0.0 { 1 } else { -1 }, page, pages) {
        0.0
    } else {
        travel
    }
}

fn end_blocks(delta: i32, page: u32, pages: u32) -> bool {
    if pages == 0 {
        return false;
    }
    (delta < 0 && page <= 1) || (delta > 0 && page >= pages)
}

/// Page and offset stored with `href` before that chapter reports its own pin.
/// A different chapter stores the jump aimed at it. A stored row with no href,
/// and the same href, keep their place.
pub fn chapter_target(
    stored_href: &str,
    href: &str,
    page: u32,
    offset: u32,
    jump_href: &str,
    jump_offset: Option<u32>,
) -> (u32, u32) {
    if stored_href.is_empty() || !changed_chapter(stored_href, href) {
        return (page, offset);
    }
    let aimed = if jump_href == href {
        jump_offset.unwrap_or(0)
    } else {
        0
    };
    (1, aimed)
}

/// One poll of an open chapter. `poll_web` applies this and then reveals or records.
pub struct ChapterPoll<'a> {
    pub saved_href: &'a str,
    pub saved_page: u32,
    pub saved_offset: u32,
    pub jump_href: &'a str,
    pub jump_offset: Option<u32>,
    pub awaiting: Option<u32>,
    pub pending: u32,
    pub hold: Option<u32>,
    pub shown: u32,
    pub href: &'a str,
    pub ready: bool,
    pub reported: bool,
    pub pinned: Option<u32>,
    pub missed: bool,
    pub sentence: u32,
    pub still_turning: bool,
}

/// Fields after `poll_chapter`. `pin_write` is a pin that replaced the stored place.
pub struct PolledChapter {
    pub saved_href: String,
    pub saved_page: u32,
    pub saved_offset: u32,
    pub jump_offset: Option<u32>,
    pub awaiting: Option<u32>,
    pub pending: u32,
    pub hold: Option<u32>,
    pub reveal_jump: Option<u32>,
    pub reveal_pending: Option<u32>,
    pub pin_write: bool,
    pub record: bool,
}

/// The place stored for a new href is the jump this poll started with.
/// Handing that jump to reveal clears the field and does not clear the place.
pub fn poll_chapter(poll: ChapterPoll<'_>) -> PolledChapter {
    let aim = poll.jump_offset;
    let mut jump_offset = poll.jump_offset;
    let mut awaiting = poll.awaiting;
    let mut pending = poll.pending;
    if jump_offset.is_some() && poll.href == poll.jump_href {
        pending = 0;
    }
    let mut reveal_jump = None;
    let mut reveal_pending = None;
    if let Some(offset) = jump_offset {
        if poll.ready && poll.href == poll.jump_href {
            jump_offset = None;
            awaiting = Some(offset);
            reveal_jump = Some(offset);
        }
    }
    if pending > 0 && poll.reported {
        let offset = pending;
        pending = 0;
        awaiting = Some(offset);
        reveal_pending = Some(offset);
    }
    let wanted = awaiting;
    let step = step_restore(
        pending > 0 || jump_offset.is_some(),
        awaiting,
        poll.hold,
        poll.shown,
        poll.pinned,
        poll.missed,
    );
    awaiting = step.want;
    let hold = step.hold;
    let mut saved_page = poll.saved_page;
    let mut saved_offset = poll.saved_offset;
    let mut saved_href = poll.saved_href.to_string();
    let mut record = false;
    let pin_write = step.write && !poll.still_turning;
    if pin_write {
        let (page, offset) = place_after_pin(
            poll.saved_page,
            poll.saved_offset,
            poll.shown,
            poll.sentence,
            wanted,
            poll.pinned,
        );
        if place_changed(
            poll.saved_page,
            poll.saved_offset,
            poll.saved_href,
            page,
            offset,
            poll.href,
        ) {
            saved_page = page;
            saved_offset = offset;
            saved_href = poll.href.to_string();
            record = true;
        }
    } else if changed_chapter(poll.saved_href, poll.href) {
        let (page, offset) = chapter_target(
            poll.saved_href,
            poll.href,
            poll.saved_page,
            poll.saved_offset,
            poll.jump_href,
            aim,
        );
        saved_page = page;
        saved_offset = offset;
        saved_href = poll.href.to_string();
        record = true;
    }
    PolledChapter {
        saved_href,
        saved_page,
        saved_offset,
        jump_offset,
        awaiting,
        pending,
        hold,
        reveal_jump,
        reveal_pending,
        pin_write,
        record,
    }
}

pub fn place_changed(
    stored_page: u32,
    stored_offset: u32,
    stored_href: &str,
    page: u32,
    offset: u32,
    href: &str,
) -> bool {
    page != stored_page || offset != stored_offset || (!href.is_empty() && href != stored_href)
}

pub fn opening_page(go: &ChapterMove, page: Option<usize>) -> Option<usize> {
    match go {
        ChapterMove::Stay => page,
        _ => None,
    }
}

pub fn place_after_pin(
    stored_page: u32,
    stored_offset: u32,
    shown: u32,
    sample: u32,
    wanted: Option<u32>,
    pinned: Option<u32>,
) -> (u32, u32) {
    if wanted.is_some() && pinned == wanted {
        return (shown.max(1), wanted.unwrap_or(0));
    }
    next_sentence(stored_page, stored_offset, shown, sample)
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

pub use spike_pdf::{
    hit_fitted_page, link_back_for_book, margin_owns_point, pdf_link_jump, pdf_user_point,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfPress {
    Follow { page: u32, back: Option<u32> },
    Stay,
    Turn(PageEdge),
}

/// A link under the click wins over an edge turn. A link that does not
/// move still keeps the edge from turning.
pub fn pdf_press(
    hit: Option<spike_pdf::LinkHit>,
    here: u32,
    pages: u32,
    back: Option<u32>,
    edge: PageEdge,
) -> PdfPress {
    match hit {
        Some(spike_pdf::LinkHit::Page(dest)) => {
            match spike_pdf::pdf_link_jump(here, dest, pages, back) {
                Some((page, back)) => PdfPress::Follow { page, back },
                None => PdfPress::Stay,
            }
        }
        Some(spike_pdf::LinkHit::Held) => PdfPress::Stay,
        None => PdfPress::Turn(edge),
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

/// One PDF page turn. `book` is the open generation. A later open makes the
/// old turn drop instead of writing the new book.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slide {
    pub book: u64,
    pub from: usize,
    pub to: usize,
    pub queued: i32,
    pub forward: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlideAt {
    /// `page` is the departing page. `shift` is how far that page has moved.
    Moving { page: usize, shift: f64 },
    /// The slide ended on `page`. `queued` is the one turn waiting behind it.
    Landed { page: usize, queued: i32 },
    /// This turn belongs to a book that is no longer open.
    Dropped,
}

/// The horizontal ease. At 80 ms of the 160 ms turn the page has moved halfway.
pub fn slide_shift(elapsed_ms: u32, width: f64) -> f64 {
    glide_shift(0.0, width.max(0.0), elapsed_ms)
}

/// Start a turn. The same page, including either end, does not start one.
pub fn begin_slide(book: u64, count: usize, index: usize, delta: i32) -> Option<Slide> {
    if delta == 0 || count == 0 {
        return None;
    }
    let index = index.min(count - 1);
    let next = step_index(index, count, i64::from(delta));
    if next == index {
        return None;
    }
    Some(Slide {
        book,
        from: index,
        to: next,
        queued: 0,
        forward: delta > 0,
    })
}

/// One waiting turn. A newer press replaces it.
pub fn queue_slide(slide: &mut Slide, delta: i32) {
    if delta == 0 {
        return;
    }
    slide.queued = if delta < 0 { -1 } else { 1 };
}

/// What the PDF pump does with one slide. Moving keeps the departing page.
/// Landed is the page to write. A changed book writes nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PdfTurn {
    Moving { page: usize },
    Landed { page: usize, queued: i32 },
    Dropped,
}

pub fn pdf_turn(slide: &Slide, elapsed_ms: u32, live_book: u64) -> PdfTurn {
    match slide_at(slide, elapsed_ms, live_book, 0.0) {
        SlideAt::Moving { page, .. } => PdfTurn::Moving { page },
        SlideAt::Landed { page, queued } => PdfTurn::Landed { page, queued },
        SlideAt::Dropped => PdfTurn::Dropped,
    }
}

/// The page a landed turn writes. The page already stored, and page 0, write nothing.
pub fn landed_progress(saved_page: u32, saved_offset: u32, shown: u32) -> Option<(u32, u32)> {
    if shown == 0 || shown == saved_page {
        return None;
    }
    Some(next_sentence(saved_page, saved_offset, shown, 0))
}

pub fn slide_at(slide: &Slide, elapsed_ms: u32, live_book: u64, width: f64) -> SlideAt {
    if slide.book != live_book {
        return SlideAt::Dropped;
    }
    if elapsed_ms >= 160 {
        return SlideAt::Landed {
            page: slide.to,
            queued: slide.queued,
        };
    }
    let sign = if slide.forward { 1.0 } else { -1.0 };
    SlideAt::Moving {
        page: slide.from,
        shift: sign * slide_shift(elapsed_ms, width),
    }
}

/// One point of finger travel is one point of page shift, clamped to the
/// fitted page. The commit distance stays 48 points. This is not a new gain.
pub fn finger_shift(travel: f64, width: f64) -> f64 {
    if !travel.is_finite() || !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    travel.clamp(-width, width)
}

/// The same cosine the 160 ms slide already uses, from the finger's offset.
pub fn glide_shift(from: f64, to: f64, elapsed_ms: u32) -> f64 {
    if elapsed_ms >= 160 {
        return to;
    }
    let u = f64::from(elapsed_ms) / 160.0;
    let ease = (1.0 - (std::f64::consts::PI * u).cos()) / 2.0;
    from + (to - from) * ease
}

fn page_index(page: u32, pages: u32) -> usize {
    if pages == 0 {
        return 0;
    }
    (page as usize).saturating_sub(1).min(pages as usize - 1)
}

fn one_page(delta: i32) -> i32 {
    if delta < 0 {
        -1
    } else if delta > 0 {
        1
    } else {
        0
    }
}

/// Positive shift reveals the next page from the right. A negative shift
/// reveals the previous page from the left. A resting key slide keeps its
/// own direction.
pub fn neighbor_on_the_right(shift: f64, key_forward: bool) -> bool {
    if shift < 0.0 {
        false
    } else if shift > 0.0 {
        true
    } else {
        key_forward
    }
}

/// The neighbor a non-zero shift reveals. A blocked end has no neighbor.
pub fn finger_neighbor(page: u32, pages: u32, shift: f64) -> Option<usize> {
    if shift == 0.0 || pages == 0 {
        return None;
    }
    let index = page_index(page, pages);
    let next = if shift > 0.0 {
        index.saturating_add(1)
    } else {
        index.saturating_sub(1)
    };
    (next != index && next < pages as usize).then_some(next)
}

/// The trackpad glide. A key slide stays a `Slide`. `write` is the page index
/// stored when the 160 ms glide lands. None returns to the page already shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FingerGlide {
    pub book: u64,
    pub page: usize,
    pub from: f64,
    pub to: f64,
    pub incoming: Option<usize>,
    pub write: Option<usize>,
    pub queued: i32,
    pub token: u64,
}

/// What one wheel effect does to the PDF picture. Follow does not write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PdfWheel {
    Pass,
    Follow { shift: f64 },
    Begin(FingerGlide),
    Busy { queued: i32 },
    Drop,
}

/// The PDF card's answer to one `Swipe::offer` effect. The stage paints it.
/// `gliding` is a commit already on screen. A return yields to a new finger.
/// A key slide keeps the picture. A second commit replaces the one queued page.
pub fn pdf_wheel(
    effect: WheelMove,
    token: u64,
    book: u64,
    page: u32,
    pages: u32,
    width: f64,
    held: f64,
    gliding: bool,
    key_slide: bool,
) -> PdfWheel {
    let busy = gliding || key_slide;
    match effect {
        WheelMove::Pass | WheelMove::Swallow | WheelMove::Hold => PdfWheel::Pass,
        WheelMove::Follow(offset) => {
            if busy {
                PdfWheel::Pass
            } else {
                PdfWheel::Follow {
                    shift: finger_shift(offset, width),
                }
            }
        }
        WheelMove::Commit(delta) => {
            let queued = one_page(delta);
            if busy {
                return PdfWheel::Busy { queued };
            }
            let index = page_index(page, pages);
            let next = step_index(index, pages as usize, i64::from(queued));
            if queued == 0 || next == index {
                return PdfWheel::Drop;
            }
            let to = if queued > 0 {
                width.max(0.0)
            } else {
                -width.max(0.0)
            };
            PdfWheel::Begin(FingerGlide {
                book,
                page: index,
                from: held,
                to,
                incoming: Some(next),
                write: Some(next),
                queued: 0,
                token,
            })
        }
        WheelMove::Ease(_) | WheelMove::Cancel => {
            if busy || held == 0.0 {
                PdfWheel::Drop
            } else {
                let index = page_index(page, pages);
                PdfWheel::Begin(FingerGlide {
                    book,
                    page: index,
                    from: held,
                    to: 0.0,
                    incoming: finger_neighbor(page, pages, held),
                    write: None,
                    queued: 0,
                    token: 0,
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlideAt {
    Moving {
        page: usize,
        shift: f64,
        incoming: Option<usize>,
    },
    Landed {
        page: usize,
        write: Option<usize>,
        queued: i32,
        token: u64,
    },
    Dropped,
}

/// The finger-up glide. A changed book writes nothing. The page is written
/// only when `elapsed_ms` reaches the existing 160 ms and `write` is set.
pub fn glide_at(glide: &FingerGlide, elapsed_ms: u32, live_book: u64) -> GlideAt {
    if glide.book != live_book {
        return GlideAt::Dropped;
    }
    if elapsed_ms >= 160 {
        return GlideAt::Landed {
            page: glide.write.unwrap_or(glide.page),
            write: glide.write,
            queued: glide.queued,
            token: glide.token,
        };
    }
    GlideAt::Moving {
        page: glide.page,
        shift: glide_shift(glide.from, glide.to, elapsed_ms),
        incoming: glide.incoming,
    }
}

/// A resize, immersive enter or leave, or a cross of 818 changes the card.
/// The finger paint does not survive that. The same card keeps it.
pub fn card_keeps_finger(old_w: f64, old_h: f64, new_w: f64, new_h: f64) -> bool {
    old_w.round() as i32 == new_w.round() as i32 && old_h.round() as i32 == new_h.round() as i32
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

    #[test]
    fn a_place_jump_keeps_the_first_origin() {
        let here = ReadingPlace {
            href: "c1.xhtml".to_string(),
            page: 2,
            offset: 10,
        };
        let dest = ReadingPlace {
            href: "c2.xhtml".to_string(),
            page: 1,
            offset: 4,
        };
        let spine = vec!["c1.xhtml".to_string(), "c2.xhtml".to_string()];
        let first = place_jump(&here, &dest, &spine, None);
        assert_eq!(first.back.as_ref(), Some(&here));
        assert!(matches!(
            first.go,
            ChapterMove::Chapter {
                index: 1,
                offset: 4
            }
        ));
        let further = ReadingPlace {
            href: "c1.xhtml".to_string(),
            page: 2,
            offset: 80,
        };
        let second = place_jump(&dest, &further, &spine, first.back.clone());
        assert_eq!(second.back, first.back);
        let same = place_jump(&here, &here, &spine, None);
        assert_eq!(same.back, None);
        assert!(matches!(same.go, ChapterMove::Stay));
    }

    fn scrolled(offset: f32) -> Visit {
        note_scroll(&Visit::shelf(), offset)
    }

    #[test]
    fn a_selection_with_a_page_is_not_a_column_report() {
        let selection = serde_json::json!({
            "kind": "selection",
            "start": 1,
            "end": 4,
            "scalar_start": 1,
            "scalar_end": 4,
            "text": "the",
            "rects": [{ "x": 1.0, "y": 2.0, "w": 3.0, "h": 4.0 }],
            "text_right": 8.0,
            "page": 2
        });
        assert_eq!(script_route(&selection), ScriptRoute::Selection);
        let report = serde_json::json!({ "kind": "box", "page": 2, "pages": 9 });
        assert_eq!(script_route(&report), ScriptRoute::Place);
        let bare = serde_json::json!({ "start": 1, "end": 4, "text": "the" });
        assert_eq!(script_route(&bare), ScriptRoute::Selection);
        let fresh = fresh_measures(&serde_json::json!([
            { "id": 4, "page": 3, "text_right": 140.0, "rects": [{ "x": 90.0, "y": 40.0, "w": 24.0, "h": 16.0 }] },
            { "id": 5, "page": 3, "text_right": 10.0 }
        ]));
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].id, 4);
        assert!((fresh[0].quads[0].x - 90.0).abs() < 0.01);
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

    #[test]
    fn a_chapter_choice_lands_on_that_sentence_and_returns() {
        let essay = fixture("Plain Essay.epub");
        assert!(essay.starts_with(b"PK"));
        let chapter = "/OEBPS/chapter.xhtml";
        let next = "/OEBPS/next.xhtml";
        let spine = [chapter.to_string(), next.to_string()];
        let marks = contents_marks("Plain Essay", &spine);
        assert_eq!(marks[0].title, "chapter.xhtml");
        assert_eq!(marks[1].title, "next.xhtml");
        assert_eq!(marks[0].offset, 0);
        let one = contents_marks("Plain Essay", &[chapter.to_string()]);
        assert_eq!(one[0].title, "Plain Essay");
        assert_eq!(one[0].href, chapter);

        let text = "It is a truth universally acknowledged, that a single man in possession of a good fortune, must be in want of a wife.";
        let netherfield = "Netherfield Park is let at last";
        let phrase = format!("{text} {netherfield}");
        let at = phrase.find("Netherfield").unwrap() as u32;
        let mut rows = one.clone();
        rows[0].offset = at;

        let here = ReadingPlace {
            href: chapter.to_string(),
            page: 4,
            offset: 800,
        };
        let jumped = chapter_jump(&here, &rows, 0, &[chapter.to_string()], None);
        assert_eq!(jumped.go, ChapterMove::Reveal { offset: at });
        assert_eq!(jumped.back.as_ref().map(|place| place.offset), Some(800));
        let back = chapter_return(
            &ReadingPlace {
                href: chapter.to_string(),
                page: 1,
                offset: at,
            },
            jumped.back,
            &[chapter.to_string()],
        );
        assert_eq!(back.go, ChapterMove::Reveal { offset: 800 });
        assert_eq!(back.back, None);
        let stored = place_after_pin(4, 800, 4, 0, Some(0), Some(0));
        assert_eq!(stored, (4, 0));
        assert_eq!(place_after_pin(4, 800, 4, 0, None, Some(0)), (4, 800));
        assert_eq!(spine_index(&spine, next), 1);
        assert_eq!(spine_index(&spine, ""), 0);
        assert_eq!(spine_index(&spine, "/missing.xhtml"), 0);
        assert_eq!(
            opened_place(&spine, "/missing.xhtml", 4, 800),
            OpenedPlace {
                index: 0,
                page: 1,
                offset: 0,
            }
        );
        assert_eq!(
            opened_place(&spine, next, 4, 800),
            OpenedPlace {
                index: 1,
                page: 4,
                offset: 800,
            }
        );
        assert_eq!(
            opened_place(&spine, "", 4, 800),
            OpenedPlace {
                index: 0,
                page: 4,
                offset: 800,
            }
        );
        let empty = "/OEBPS/empty.xhtml";
        let missed_chapter = step_restore(false, Some(0), None, 1, Some(0), true);
        assert!(!missed_chapter.write);
        assert_eq!(missed_chapter.hold, Some(1));
        assert!(changed_chapter(chapter, empty));
        assert!(!changed_chapter(empty, empty));
        assert!(!changed_chapter(chapter, ""));
        let origin = ReadingPlace {
            href: chapter.to_string(),
            page: 4,
            offset: 800,
        };
        let chosen = chapter_jump(&origin, &marks, 1, &spine, None);
        let ChapterMove::Chapter { index, offset } = chosen.go else {
            panic!("a different chapter is a chapter move");
        };
        assert_eq!(offset, 0);
        assert_eq!(
            chapter_target(chapter, &spine[index], 4, 800, &spine[index], Some(offset)),
            (1, 0)
        );
        let returned = chapter_return(
            &ReadingPlace {
                href: next.to_string(),
                page: 1,
                offset: 0,
            },
            Some(origin.clone()),
            &spine,
        );
        let ChapterMove::Chapter {
            index: back_index,
            offset: back_offset,
        } = returned.go
        else {
            panic!("returning across chapters is a chapter move");
        };
        assert_eq!(back_offset, 800);
        let ready_first = poll_chapter(ChapterPoll {
            saved_href: next,
            saved_page: 1,
            saved_offset: 0,
            jump_href: &spine[back_index],
            jump_offset: Some(back_offset),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: &spine[back_index],
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!(ready_first.reveal_jump, Some(800));
        assert_eq!(ready_first.jump_offset, None);
        assert_eq!(ready_first.saved_offset, 800);
        let again = chapter_jump(
            &ReadingPlace {
                href: ready_first.saved_href,
                page: ready_first.saved_page,
                offset: ready_first.saved_offset,
            },
            &marks,
            1,
            &spine,
            None,
        );
        assert_eq!(again.back.as_ref().map(|place| place.offset), Some(800));
        assert_eq!(
            chapter_target(chapter, next, 4, 800, chapter, Some(800)),
            (1, 0)
        );
        assert_eq!(
            chapter_target(chapter, empty, 4, 800, empty, Some(0)),
            (1, 0)
        );
        assert_eq!(
            chapter_target(chapter, chapter, 4, 800, chapter, Some(at)),
            (4, 800)
        );
        assert_eq!(chapter_target(chapter, "", 4, 800, "", None), (4, 800));
        assert_eq!(
            chapter_target("", chapter, 4, 800, chapter, Some(0)),
            (4, 800)
        );
        let waiting = step_restore(true, None, None, 4, None, false);
        assert!(!waiting.write);
        assert!(place_changed(1, 0, chapter, 1, 0, next));
        assert!(!place_changed(1, 0, next, 1, 0, next));
        assert!(!place_changed(4, 800, chapter, 4, 800, ""));
        assert_eq!(
            opening_page(
                &ChapterMove::Chapter {
                    index: 1,
                    offset: 0
                },
                Some(4)
            ),
            None
        );
        assert_eq!(opening_page(&ChapterMove::Stay, Some(4)), Some(4));

        let other = chapter_jump(&here, &marks, 1, &spine, None);
        assert_eq!(
            other.go,
            ChapterMove::Chapter {
                index: 1,
                offset: 0
            }
        );
        assert_eq!(other.back.as_ref().map(|place| place.page), Some(4));
        let missing = chapter_jump(&here, &marks, 9, &spine, other.back.clone());
        assert_eq!(missing.go, ChapterMove::Stay);
        assert_eq!(missing.back, other.back);
        let lost = ChapterMark {
            title: "gone".into(),
            href: "/missing.xhtml".into(),
            offset: 0,
            page: 0,
        };
        let stayed = chapter_jump(&here, &[lost], 0, &spine, None);
        assert_eq!(stayed.go, ChapterMove::Stay);
        assert_eq!(stayed.back, None);

        let pdf_here = ReadingPlace {
            href: String::new(),
            page: 1,
            offset: 0,
        };
        let pdf = ChapterMark {
            title: "Field".into(),
            href: String::new(),
            offset: 0,
            page: 3,
        };
        let turned = chapter_jump(&pdf_here, &[pdf], 0, &[], None);
        assert_eq!(turned.go, ChapterMove::Page { page: 3 });
        let same = chapter_return(
            &ReadingPlace {
                href: String::new(),
                page: 3,
                offset: 0,
            },
            turned.back,
            &[],
        );
        assert_eq!(same.go, ChapterMove::Page { page: 1 });
        assert_eq!(same.back, None);
        assert_eq!(chapter_return(&here, None, &spine).go, ChapterMove::Stay);
        let kept = chapter_return(
            &here,
            Some(ReadingPlace {
                href: "/missing.xhtml".into(),
                page: 1,
                offset: 3,
            }),
            &spine,
        );
        assert_eq!(kept.go, ChapterMove::Stay);
        assert_eq!(kept.back.unwrap().offset, 3);
        publish_surface(Surface::Library);
        assert!(on_library());
        publish_surface(Surface::Reading);
        assert!(!on_library());
        publish_surface(Surface::Library);
        assert!(on_library());
    }

    #[test]
    fn a_trackpad_swipe_follows_cancels_and_commits_once() {
        let down = |dx: f64| WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let mom = |dx: f64| WheelTick {
            finger: Finger::None,
            momentum: true,
            dx,
            dy: 0.0,
        };
        let writes = |effect: WheelMove| matches!(effect, WheelMove::Commit(_));

        let mut swipe = Swipe::default();
        let small = swipe.offer(down(10.0), 4, 10);
        assert_eq!(small, WheelMove::Follow(10.0));
        assert!(!writes(small));
        let cancelled = swipe.offer(up, 4, 10);
        assert_eq!(cancelled, WheelMove::Cancel);
        assert!(!writes(cancelled));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Pass);

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(40.0), 4, 10), WheelMove::Follow(40.0));
        assert_eq!(swipe.offer(down(-30.0), 4, 10), WheelMove::Follow(10.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Cancel);

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(60.0), 4, 10), WheelMove::Follow(60.0));
        assert_eq!(swipe.offer(down(-120.0), 4, 10), WheelMove::Follow(-60.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(-1));
        assert_eq!(swipe.offer(mom(200.0), 5, 10), WheelMove::Swallow);
        assert_eq!(swipe.offer(mom(200.0), 5, 10), WheelMove::Swallow);

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(30.0), 4, 10), WheelMove::Follow(30.0));
        assert_eq!(swipe.offer(down(18.0), 4, 10), WheelMove::Follow(48.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        assert_eq!(
            swipe.offer(down(SWIPE_POINTS), 5, 10),
            WheelMove::Follow(SWIPE_POINTS)
        );
        assert_eq!(swipe.offer(up, 5, 10), WheelMove::Commit(1));

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert!(swipe.abandon());
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Pass);
        assert_eq!(swipe.offer(down(80.0), 2, 10), WheelMove::Follow(80.0));
        assert!(swipe.abandon());
        assert_eq!(swipe.offer(mom(80.0), 2, 10), WheelMove::Swallow);

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(-80.0), 1, 10), WheelMove::Follow(0.0));
        let eased = swipe.offer(up, 1, 10);
        assert_eq!(eased, WheelMove::Ease(-1));
        assert!(!writes(eased));
        assert_eq!(swipe.offer(down(80.0), 10, 10), WheelMove::Follow(0.0));
        assert_eq!(swipe.offer(up, 10, 10), WheelMove::Ease(1));
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(-10.0), 1, 10), WheelMove::Follow(0.0));
        assert_eq!(swipe.offer(up, 1, 10), WheelMove::Cancel);
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(-80.0), 1, 10), WheelMove::Follow(0.0));
        assert_eq!(
            swipe.offer(
                WheelTick {
                    finger: Finger::Cancelled,
                    momentum: false,
                    dx: 0.0,
                    dy: 0.0,
                },
                1,
                10,
            ),
            WheelMove::Cancel
        );

        let mut swipe = Swipe::default();
        assert_eq!(
            swipe.offer(
                WheelTick {
                    finger: Finger::Down,
                    momentum: false,
                    dx: 4.0,
                    dy: 20.0,
                },
                4,
                10,
            ),
            WheelMove::Pass
        );
        let held = swipe.offer(
            WheelTick {
                finger: Finger::None,
                momentum: false,
                dx: 20.0,
                dy: 0.0,
            },
            4,
            10,
        );
        assert_eq!(held, WheelMove::Hold);
        assert!(!writes(held));
        assert_eq!(
            swipe.offer(
                WheelTick {
                    finger: Finger::None,
                    momentum: false,
                    dx: 28.0,
                    dy: 0.0,
                },
                4,
                10,
            ),
            WheelMove::Commit(1)
        );
        assert_eq!(
            swipe.offer(
                WheelTick {
                    finger: Finger::None,
                    momentum: false,
                    dx: -SWIPE_POINTS,
                    dy: 0.0,
                },
                1,
                10,
            ),
            WheelMove::Cancel
        );

        assert_eq!(finger_bits(4), Finger::Down);
        assert_eq!(finger_bits(8), Finger::Up);
        assert_eq!(finger_bits(16), Finger::Cancelled);
        assert_eq!(finger_bits(0), Finger::None);
        assert_eq!(wheel_points(true, 12.0, 1.0), 12.0);
        assert_eq!(wheel_points(false, 0.0, 1.0), 16.0);
    }

    #[test]
    fn a_release_stays_open_until_chapter_or_resize_cancels_it() {
        let down = WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx: 80.0,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down, 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        assert!(swipe.release_open());
        assert!(!swipe.abandon());
        assert!(swipe.release_open());
        assert!(swipe.cancel_release());
        assert!(!swipe.release_open());
        assert!(!swipe.cancel_release());

        let mut swipe = Swipe::default();
        assert_eq!(
            swipe.offer(
                WheelTick {
                    finger: Finger::Down,
                    momentum: false,
                    dx: 10.0,
                    dy: 0.0,
                },
                4,
                10,
            ),
            WheelMove::Follow(10.0)
        );
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Cancel);
        assert!(swipe.release_open());
        assert!(swipe.cancel_release());

        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down, 4, 10), WheelMove::Follow(80.0));
        assert!(swipe.cancel_release());
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Pass);

        let mut notch = Swipe::default();
        assert_eq!(
            notch.offer(
                WheelTick {
                    finger: Finger::None,
                    momentum: false,
                    dx: SWIPE_POINTS,
                    dy: 0.0,
                },
                4,
                10,
            ),
            WheelMove::Commit(1)
        );
        assert!(notch.release_open());
        assert!(!notch.abandon());
        assert!(notch.cancel_release());

        assert_eq!(place_step(true, true, true), PlaceStep::Hold);
        assert_eq!(
            place_step(true, true, false),
            PlaceStep::Cancel { reflow: false }
        );
        assert_eq!(
            place_step(false, true, true),
            PlaceStep::Cancel { reflow: true }
        );
        let mut reader = Reader::default();
        reader.walk = Some(10);
        let step = place_step(true, true, false);
        assert_eq!(step, PlaceStep::Cancel { reflow: false });
        let command = cancel_placed_page(&mut reader);
        assert_eq!(reader.walk, Some(10));
        assert!(command.contains("spikeFollowDrop"));
        assert!(!reader.swipe.release_open());
        let mut tick = FrameTick::new(false, true, true, true, true, false);
        tick.shown = 4;
        tick.pages = 40;
        let started = layout_frame(&mut reader, tick);
        let Draw::Measure { page: 10, probe } = started.draw else {
            panic!("measure the saved column, {started:?}");
        };
        assert!(started.cancel);
        assert!(started.pump);
        assert_eq!(probe, reader.probe);
        tick.awaiting = true;
        tick.hold_probe = probe;
        let waiting = layout_frame(&mut reader, tick);
        assert_eq!(waiting.draw, Draw::Rest);
        assert!(!waiting.cancel);
        assert!(waiting.pump);
        assert_eq!(reader.probe, probe);
        tick.timed_out = true;
        let retry = layout_frame(&mut reader, tick);
        let Draw::Measure {
            page: 10,
            probe: retried,
        } = retry.draw
        else {
            panic!("the first timeout samples the same column, {retry:?}");
        };
        assert!(!retry.cancel);
        assert!(retry.pump);
        assert_ne!(retried, probe);
        assert_eq!(reader.sample_misses, 1);
        assert_eq!(reader.walk, Some(10));
        tick.timed_out = false;
        tick.hold_probe = retried;
        let still_waiting = layout_frame(&mut reader, tick);
        assert_eq!(still_waiting.draw, Draw::Rest);
        assert!(!still_waiting.cancel);
        assert!(still_waiting.pump);
        tick.hold = Some(360);
        let found = layout_frame(&mut reader, tick);
        assert_eq!(found.draw, Draw::Move { reveal: Some(360) });
        assert!(!found.cancel);
        assert_eq!(reader.sample_misses, 0);
        reader.measured = None;
        reader.anchoring = true;
        reader.hold_layout = true;
        reader.sample_misses = 1;
        reader.probe = retried;
        tick.hold = None;
        tick.timed_out = true;
        let failed = layout_frame(&mut reader, tick);
        assert_eq!(failed.draw, Draw::Move { reveal: None });
        assert!(!failed.cancel);
        assert!(!failed.pump);
        assert!(reader.hold_layout);
        assert_eq!(reader.sample_misses, 2);
        assert!(reader.measured.is_none());
        assert_eq!(reader.walk, Some(10));
        assert!(current_hold(Some(42), probe, reader.probe).is_none());
        let polled = poll_anchored(&mut reader, 4, 40, false);
        assert!(polled.command.is_none());
        assert_eq!(polled.walk, Some(10));
        tick.awaiting = false;
        tick.timed_out = false;
        tick.fitted_same = true;
        let mut cancels = 0;
        let mut pumps = 0;
        for _ in 0..8 {
            let rested = layout_frame(&mut reader, tick);
            assert_eq!(rested.draw, Draw::Rest);
            if rested.cancel {
                cancels += 1;
            }
            if rested.pump {
                pumps += 1;
            }
            assert_eq!(reader.walk, Some(10));
        }
        assert_eq!(cancels, 0);
        assert_eq!(pumps, 0);
        for _ in 0..3 {
            tick.fitted_same = false;
            let again = layout_frame(&mut reader, tick);
            assert_eq!(again.draw, Draw::Move { reveal: None }, "{again:?}");
            assert!(!again.cancel);
            assert!(!again.pump);
            assert_eq!(reader.walk, Some(10));
            tick.fitted_same = true;
        }
        assert_eq!(reader.walk, Some(10));
        let mut fresh = Reader::default();
        fresh.walk = Some(10);
        let mut open = FrameTick::new(false, true, true, true, false, false);
        open.shown = 4;
        open.pages = 40;
        fresh.measured = Some(0);
        let zero_open = layout_frame(&mut fresh, open);
        assert_eq!(zero_open.draw, Draw::Move { reveal: Some(0) });
        fresh.measured = Some(360);
        fresh.anchoring = false;
        let text_open = layout_frame(&mut fresh, open);
        assert_eq!(text_open.draw, Draw::Move { reveal: Some(360) });
        fresh.measured = None;
        fresh.sample_misses = 0;
        fresh.anchoring = false;
        fresh.hold_layout = false;
        let unloaded = layout_frame(&mut fresh, open);
        assert_eq!(unloaded.draw, Draw::Move { reveal: None });
        assert_eq!(fresh.sample_misses, 0);
        open.settled = true;
        let loaded = layout_frame(&mut fresh, open);
        let Draw::Measure { page: 10, .. } = loaded.draw else {
            panic!("an unsettled book is not a lost book, {loaded:?}");
        };
        assert!(loaded.cancel);
        fresh.figure = Some("img#pic".to_string());
        let mut lost_figure = open;
        lost_figure.awaiting = true;
        lost_figure.figure_lost = true;
        let looked = layout_frame(&mut fresh, lost_figure);
        assert_eq!(looked.draw, Draw::Align);
        assert_eq!(fresh.figure_misses, 1);
        let gone = layout_frame(&mut fresh, lost_figure);
        assert_eq!(gone.draw, Draw::Move { reveal: None });
        assert!(fresh.figure.is_none());
        assert_eq!(fresh.sample_misses, 2);
        lost_figure.awaiting = false;
        lost_figure.figure_lost = false;
        lost_figure.fitted_same = false;
        let after = layout_frame(&mut fresh, lost_figure);
        assert_eq!(after.draw, Draw::Move { reveal: None });
        reader.measured = Some(42);
        reader.anchoring = false;
        tick.fitted_same = false;
        let known = layout_frame(&mut reader, tick);
        assert_eq!(known.draw, Draw::Move { reveal: Some(42) });
        assert!(!known.cancel);
        assert!(!known.pump);
        assert!(reader.anchoring);
        assert_eq!(reader.walk, Some(10));
        reader.measured = Some(0);
        let zero = layout_frame(&mut reader, tick);
        assert_eq!(zero.draw, Draw::Move { reveal: Some(0) });
        assert!(!zero.pump);
        assert!(reader.anchoring);
        assert_eq!(reader.walk, Some(10));
    }

    #[test]
    fn a_delayed_reveal_does_not_let_the_lift_replace_the_pin() {
        let down = |dx: f64| WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        // The finger is down when poll reveals. Reveal retires that finger,
        // so the lift is not a commit.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert!(swipe.cancel_release());
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Pass);

        // The commit was already handed to the page. Reveal retires its token.
        // The glide's post can still arrive after that, and it must not apply.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let token = swipe.commit_token().expect("commit");
        assert!(swipe.cancel_release());
        assert_eq!(swipe_progress(&mut swipe, Some(token), 2, 50), None);
        let kept = poll_chapter(ChapterPoll {
            saved_href: "ch",
            saved_page: 1,
            saved_offset: 800,
            jump_href: "",
            jump_offset: None,
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: "ch",
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 800,
            still_turning: false,
        });
        assert_eq!((kept.saved_page, kept.saved_offset), (1, 800));
        // The same report, if the handler had applied it, replaces the pin.
        let replaced = poll_chapter(ChapterPoll {
            saved_href: "ch",
            saved_page: 1,
            saved_offset: 800,
            jump_href: "",
            jump_offset: None,
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 2,
            href: "ch",
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 50,
            still_turning: false,
        });
        assert_eq!((replaced.saved_page, replaced.saved_offset), (2, 50));
        assert!(replaced.record);

        // A pin report does not retire the commit. The reveal command does.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let token = swipe.commit_token().expect("commit");
        assert_eq!(swipe_progress(&mut swipe, None, 1, 800), Some((1, 800)));
        assert_eq!(
            swipe_progress(&mut swipe, Some(token), 2, 50),
            Some((2, 50))
        );

        // Pin first, then the retired post.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let token = swipe.commit_token().expect("commit");
        assert!(swipe.cancel_release());
        assert_eq!(swipe_progress(&mut swipe, None, 1, 800), Some((1, 800)));
        assert_eq!(swipe_progress(&mut swipe, Some(token), 2, 50), None);

        // Post first, then the pin.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let token = swipe.commit_token().expect("commit");
        assert!(swipe.cancel_release());
        assert_eq!(swipe_progress(&mut swipe, Some(token), 2, 50), None);
        assert_eq!(swipe_progress(&mut swipe, None, 1, 800), Some((1, 800)));
    }

    #[test]
    fn a_second_finger_key_keeps_the_committed_page() {
        let down = |dx: f64| WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let token = swipe.commit_token().expect("commit");
        assert_eq!(swipe.offer(down(40.0), 4, 10), WheelMove::Follow(40.0));
        assert!(!swipe.abandon());
        assert!(swipe.release_open());
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Pass);
        assert_eq!(swipe_progress(&mut swipe, Some(token), 2, 0), Some((2, 0)));
        assert!(!swipe.release_open());
        assert_eq!(swipe.commit_token(), None);

        // A second commit does not retire the first post, and navigation retires both.
        let mut swipe = Swipe::default();
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let first = swipe.commit_token().expect("first");
        assert_eq!(swipe.offer(down(80.0), 4, 10), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 10), WheelMove::Commit(1));
        let second = swipe.commit_token().expect("second");
        assert_ne!(first, second);
        assert_eq!(swipe_progress(&mut swipe, Some(first), 2, 0), Some((2, 0)));
        assert!(swipe.release_open());
        assert!(swipe.cancel_release());
        assert_eq!(swipe_progress(&mut swipe, Some(second), 3, 0), None);
    }

    #[test]
    fn a_long_chapter_commit_is_never_dropped() {
        let down = WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx: 80.0,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let mut swipe = Swipe::default();
        let mut view = PageView::default();
        for n in 1u64..=128 {
            assert_eq!(swipe.offer(down, 4, 400), WheelMove::Follow(80.0));
            assert_eq!(swipe.offer(up, 4, 400), WheelMove::Commit(1));
            let token = swipe.commit_token().expect("token");
            assert_eq!(token, n);
            swipe.note_answer(token, "glide");
            let page = n as u32;
            assert!(deliver_page(
                &mut swipe,
                &mut view,
                Some(token),
                page,
                400,
                Some(0),
                None,
                None
            ));
            assert_eq!(view.page, Some(page));
        }
        let again = swipe.commit_token();
        assert_eq!(again, None);
        assert_eq!(swipe.offer(down, 4, 400), WheelMove::Follow(80.0));
        assert_eq!(swipe.offer(up, 4, 400), WheelMove::Commit(1));
        let late = swipe.commit_token().expect("late");
        assert!(swipe.cancel_release());
        assert!(!deliver_page(
            &mut swipe,
            &mut view,
            Some(late),
            2,
            400,
            Some(50),
            None,
            None
        ));
        assert_eq!(view.page, Some(128));
        assert!(deliver_page(
            &mut swipe,
            &mut view,
            None,
            1,
            400,
            Some(800),
            None,
            None
        ));
        assert_eq!((view.page, view.sentence), (Some(1), Some(800)));
        assert!(!deliver_page(
            &mut swipe,
            &mut view,
            Some(1),
            2,
            400,
            Some(0),
            None,
            None
        ));
        assert_eq!(view.page, Some(1));
    }

    #[test]
    fn a_finished_page_is_saved_before_chapter_or_resize() {
        let down = WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx: 80.0,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let issue = |swipe: &mut Swipe| {
            assert_eq!(swipe.offer(down, 4, 400), WheelMove::Follow(80.0));
            assert_eq!(swipe.offer(up, 4, 400), WheelMove::Commit(1));
            swipe.commit_token().expect("token")
        };
        let saved_of = |view: &PageView| {
            poll_chapter(ChapterPoll {
                saved_href: "ch.xhtml",
                saved_page: 1,
                saved_offset: 0,
                jump_href: "",
                jump_offset: None,
                awaiting: None,
                pending: 0,
                hold: None,
                shown: view.page.unwrap_or(1),
                href: "ch.xhtml",
                ready: true,
                reported: view.page.is_some(),
                pinned: view.pinned,
                missed: view.pin_missed,
                sentence: view.sentence.unwrap_or(0),
                still_turning: false,
            })
        };

        let mut missed = Swipe::default();
        let mut view = PageView {
            page: Some(1),
            pages: Some(10),
            sentence: Some(0),
            pin_missed: true,
            pinned: Some(800),
        };
        assert!(!deliver_page(
            &mut missed,
            &mut view,
            Some(9),
            4,
            10,
            Some(50),
            None,
            Some(1)
        ));
        assert!(view.pin_missed);
        assert_eq!(view.pinned, Some(800));
        assert_eq!(view.page, Some(1));
        let still_missed = poll_chapter(ChapterPoll {
            saved_href: "ch.xhtml",
            saved_page: 1,
            saved_offset: 0,
            jump_href: "ch.xhtml",
            jump_offset: None,
            awaiting: Some(800),
            pending: 0,
            hold: None,
            shown: 1,
            href: "ch.xhtml",
            ready: true,
            reported: true,
            pinned: view.pinned,
            missed: view.pin_missed,
            sentence: 0,
            still_turning: false,
        });
        assert!(!still_missed.pin_write);
        assert!(!still_missed.record);

        let mut swipe = Swipe::default();
        let first = issue(&mut swipe);
        swipe.note_answer(first, "glide");
        assert!(deliver_page(
            &mut swipe,
            &mut view,
            Some(first),
            2,
            10,
            Some(100),
            None,
            None
        ));
        let stored = saved_of(&view);
        assert!(stored.record);
        assert_eq!((stored.saved_page, stored.saved_offset), (2, 100));
        assert!(stored.reveal_jump.is_none());
        swipe.cancel_release();
        assert_eq!((view.page, view.sentence), (Some(2), Some(100)));

        let second = issue(&mut swipe);
        swipe.note_answer(second, "glide");
        assert!(swipe.cancel_release());
        assert!(!deliver_page(
            &mut swipe,
            &mut view,
            Some(second),
            3,
            10,
            Some(200),
            None,
            None
        ));
        assert_eq!((view.page, view.sentence), (Some(2), Some(100)));

        let mut ordered = Swipe::default();
        let mut pages = PageView::default();
        let one = issue(&mut ordered);
        ordered.note_answer(one, "glide");
        let two = issue(&mut ordered);
        ordered.note_answer(two, "queued");
        assert!(deliver_page(
            &mut ordered,
            &mut pages,
            Some(one),
            2,
            10,
            Some(0),
            None,
            None
        ));
        assert!(deliver_page(
            &mut ordered,
            &mut pages,
            Some(two),
            3,
            10,
            Some(0),
            None,
            None
        ));
        assert_eq!(pages.page, Some(3));

        let mut late_old = Swipe::default();
        let mut kept = PageView::default();
        let older = issue(&mut late_old);
        late_old.note_answer(older, "glide");
        let newer = issue(&mut late_old);
        late_old.note_answer(newer, "queued");
        assert!(deliver_page(
            &mut late_old,
            &mut kept,
            Some(newer),
            3,
            10,
            Some(0),
            None,
            None
        ));
        assert!(!deliver_page(
            &mut late_old,
            &mut kept,
            Some(older),
            2,
            10,
            Some(0),
            None,
            None
        ));
        assert_eq!(kept.page, Some(3));

        let mut retired = Swipe::default();
        let mut revealed = PageView {
            page: Some(1),
            pages: Some(10),
            sentence: Some(800),
            pin_missed: false,
            pinned: Some(800),
        };
        let stale = issue(&mut retired);
        retired.note_answer(stale, "glide");
        assert!(retired.cancel_release());
        assert!(!deliver_page(
            &mut retired,
            &mut revealed,
            Some(stale),
            4,
            10,
            Some(50),
            None,
            None
        ));
        assert_eq!((revealed.page, revealed.sentence), (Some(1), Some(800)));
    }

    #[test]
    fn a_pdf_slide_keeps_the_departing_page_until_it_lands() {
        assert!(begin_slide(1, 3, 2, 1).is_none());
        assert!(begin_slide(1, 3, 0, -1).is_none());
        let mut slide = begin_slide(7, 75, 0, 1).unwrap();
        match slide_at(&slide, 80, 7, 200.0) {
            SlideAt::Moving { page, shift } => {
                assert_eq!(page, 0);
                assert!((shift - 100.0).abs() < 1e-6);
            }
            other => panic!("halfway frame moved the page: {other:?}"),
        }
        assert_eq!(
            slide_at(&slide, 159, 7, 200.0),
            SlideAt::Moving {
                page: 0,
                shift: slide_shift(159, 200.0)
            }
        );
        queue_slide(&mut slide, 1);
        queue_slide(&mut slide, -1);
        assert_eq!(
            slide_at(&slide, 160, 7, 200.0),
            SlideAt::Landed {
                page: 1,
                queued: -1
            }
        );
        assert_eq!(slide_at(&slide, 80, 8, 200.0), SlideAt::Dropped);
        let again = begin_slide(8, 75, 1, -1).unwrap();
        assert_eq!(again.from, 1);
        assert_eq!(again.to, 0);
        assert!(!again.forward);
    }

    #[test]
    fn a_pdf_turn_writes_the_landed_page_and_drops_a_changed_book() {
        let dir = std::env::temp_dir().join(format!("superreader-pdf-turn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/field.pdf";
        crate::settings::record_open(&file, book, 1, Some(75), 1).unwrap();

        let mut slide = begin_slide(3, 75, 0, 1).unwrap();
        assert_eq!(pdf_turn(&slide, 80, 3), PdfTurn::Moving { page: 0 });
        assert!(landed_progress(1, 0, 1).is_none());
        assert_eq!(crate::settings::resume_page(&file, book), 1);

        queue_slide(&mut slide, 1);
        queue_slide(&mut slide, -1);
        let PdfTurn::Landed { page, queued } = pdf_turn(&slide, 160, 3) else {
            panic!("the turn did not land");
        };
        assert_eq!((page, queued), (1, -1));
        let (written, offset) = landed_progress(1, 0, page as u32 + 1).unwrap();
        assert_eq!((written, offset), (2, 0));
        crate::settings::record_progress(&file, book, written, Some(75), offset, "").unwrap();
        let store = crate::settings::SettingsBook { file: &file, book };
        assert_eq!(store.resume().page, 2);
        assert_eq!(crate::settings::resume_page(&file, book), 2);

        let back = begin_slide(3, 75, page, queued).unwrap();
        assert_eq!(pdf_turn(&back, 80, 4), PdfTurn::Dropped);
        assert!(landed_progress(2, 0, 2).is_none());
        assert_eq!(store.resume().page, 2);

        let hidden = crate::form::reading_frame(817.0, 800.0, 0.0, 0.0, false);
        let covered = crate::form::reading_frame(817.0, 800.0, 0.0, 0.0, true);
        assert_eq!(hidden.card, covered.card);
        assert!(covered.covers);
        let column = crate::form::reading_frame(818.0, 800.0, 0.0, 0.0, true);
        assert!(!column.covers);
        let column_hidden = crate::form::reading_frame(818.0, 800.0, 0.0, 0.0, false);
        assert!((column_hidden.card.w - column.card.w - crate::form::MARGIN_W).abs() < 0.1);
        let returned = crate::form::reading_frame(817.0, 800.0, 0.0, 0.0, true);
        assert_eq!(returned.card, hidden.card);
        assert!(returned.covers);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_finger_moves_the_pdf_page_and_saves_when_the_glide_lands() {
        let dir =
            std::env::temp_dir().join(format!("superreader-pdf-finger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/field.pdf";
        crate::settings::record_open(&file, book, 4, Some(75), 1).unwrap();
        let store = crate::settings::SettingsBook { file: &file, book };
        let width = 400.0;
        let down = |dx: f64| WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let mom = |dx: f64| WheelTick {
            finger: Finger::None,
            momentum: true,
            dx,
            dy: 0.0,
        };

        let mut swipe = Swipe::default();
        let first = swipe.offer(down(30.0), 4, 75);
        let PdfWheel::Follow { shift } = pdf_wheel(first, 0, 1, 4, 75, width, 0.0, false, false)
        else {
            panic!("the finger did not move the page");
        };
        assert_eq!(shift, 30.0);
        assert_eq!(store.resume().page, 4);

        let second = swipe.offer(down(50.0), 4, 75);
        let PdfWheel::Follow { shift } = pdf_wheel(second, 0, 1, 4, 75, width, shift, false, false)
        else {
            panic!("the page left the finger");
        };
        assert_eq!(shift, 80.0);
        assert!(landed_progress(4, 0, 4).is_none());
        assert_eq!(store.resume().page, 4);

        let lift = swipe.offer(up, 4, 75);
        assert_eq!(lift, WheelMove::Commit(1));
        let token = swipe.commit_token().unwrap();
        let mut glide = match pdf_wheel(lift, token, 1, 4, 75, width, 80.0, false, false) {
            PdfWheel::Begin(glide) => glide,
            other => panic!("the lift did not start from the finger: {other:?}"),
        };
        assert_eq!(glide.from, 80.0);
        assert_eq!(glide.to, width);
        assert_eq!(glide.write, Some(4));
        match glide_at(&glide, 0, 1) {
            GlideAt::Moving {
                page,
                shift,
                incoming,
            } => {
                assert_eq!((page, incoming), (3, Some(4)));
                assert!((shift - 80.0).abs() < 0.01);
            }
            other => panic!("the glide left the finger: {other:?}"),
        }
        let mid = glide_shift(80.0, width, 80);
        match glide_at(&glide, 80, 1) {
            GlideAt::Moving { shift, .. } => assert!((shift - mid).abs() < 0.01),
            other => panic!("the glide skipped the existing ease: {other:?}"),
        }
        assert!((slide_shift(80, width) - glide_shift(0.0, width, 80)).abs() < 0.01);
        assert_eq!(store.resume().page, 4);
        assert_eq!(swipe.commit_token(), Some(token));

        let GlideAt::Landed {
            write,
            token: landed_token,
            queued,
            page,
        } = glide_at(&glide, 160, 1)
        else {
            panic!("the glide did not land");
        };
        assert_eq!((page, write, queued, landed_token), (4, Some(4), 0, token));
        assert!(swipe.take_commit_post(landed_token));
        let (written, offset) = landed_progress(4, 0, page as u32 + 1).unwrap();
        assert_eq!((written, offset), (5, 0));
        crate::settings::record_progress(&file, book, written, Some(75), offset, "").unwrap();
        assert_eq!(store.resume().page, 5);

        let mut back = Swipe::default();
        let held = match pdf_wheel(
            back.offer(down(20.0), 5, 75),
            0,
            1,
            5,
            75,
            width,
            0.0,
            false,
            false,
        ) {
            PdfWheel::Follow { shift } => shift,
            other => panic!("a short finger did not paint: {other:?}"),
        };
        assert_eq!(held, 20.0);
        let short = back.offer(up, 5, 75);
        assert_eq!(short, WheelMove::Cancel);
        let returning = match pdf_wheel(short, 0, 1, 5, 75, width, held, false, false) {
            PdfWheel::Begin(glide) => glide,
            other => panic!("a short lift did not return: {other:?}"),
        };
        assert_eq!(returning.write, None);
        assert_eq!(returning.to, 0.0);
        match glide_at(&returning, 160, 1) {
            GlideAt::Landed { write, .. } => assert_eq!(write, None),
            other => panic!("the return wrote a page: {other:?}"),
        }
        assert_eq!(store.resume().page, 5);
        let again = back.offer(down(15.0), 5, 75);
        assert_eq!(again, WheelMove::Follow(15.0));
        assert_eq!(
            pdf_wheel(again, 0, 1, 5, 75, width, 0.0, false, false),
            PdfWheel::Follow { shift: 15.0 }
        );

        let mut edge = Swipe::default();
        let blocked = edge.offer(down(-80.0), 1, 75);
        assert_eq!(blocked, WheelMove::Follow(0.0));
        assert!(matches!(
            pdf_wheel(blocked, 0, 1, 1, 75, width, 0.0, false, false),
            PdfWheel::Follow { shift: 0.0 }
        ));
        let eased = edge.offer(up, 1, 75);
        assert_eq!(eased, WheelMove::Ease(-1));
        assert_eq!(
            pdf_wheel(eased, 0, 1, 1, 75, width, 0.0, false, false),
            PdfWheel::Drop
        );
        assert_eq!(store.resume().page, 5);

        let mut reversed = Swipe::default();
        assert_eq!(reversed.offer(down(30.0), 5, 75), WheelMove::Follow(30.0));
        assert_eq!(reversed.offer(down(-50.0), 5, 75), WheelMove::Follow(-20.0));
        assert_eq!(reversed.offer(up, 5, 75), WheelMove::Cancel);
        let mut turned = Swipe::default();
        assert_eq!(turned.offer(down(30.0), 5, 75), WheelMove::Follow(30.0));
        assert_eq!(turned.offer(down(-100.0), 5, 75), WheelMove::Follow(-70.0));
        let back_page = turned.offer(up, 5, 75);
        assert_eq!(back_page, WheelMove::Commit(-1));
        match pdf_wheel(
            back_page,
            turned.commit_token().unwrap(),
            1,
            5,
            75,
            width,
            -70.0,
            false,
            false,
        ) {
            PdfWheel::Begin(glide) => {
                assert_eq!(glide.to, -width);
                assert_eq!(glide.write, Some(3));
                match glide_at(&glide, 0, 1) {
                    GlideAt::Moving { shift, .. } => {
                        assert!(shift < 0.0);
                        assert!(!neighbor_on_the_right(shift, true));
                    }
                    other => panic!("the previous page did not enter from the left: {other:?}"),
                }
            }
            other => panic!("a reversal did not turn back: {other:?}"),
        }
        assert_eq!(store.resume().page, 5);

        let mut coast = Swipe::default();
        assert_eq!(coast.offer(down(48.0), 5, 75), WheelMove::Follow(48.0));
        assert_eq!(coast.offer(up, 5, 75), WheelMove::Commit(1));
        assert_eq!(coast.offer(mom(200.0), 6, 75), WheelMove::Swallow);
        assert_eq!(
            pdf_wheel(WheelMove::Swallow, 0, 1, 6, 75, width, 0.0, true, false),
            PdfWheel::Pass
        );
        assert_eq!(store.resume().page, 5);

        let queued_finger = coast.offer(down(80.0), 6, 75);
        assert_eq!(
            pdf_wheel(queued_finger, 0, 1, 6, 75, width, 0.0, true, false),
            PdfWheel::Pass
        );
        let queued_lift = coast.offer(up, 6, 75);
        assert_eq!(queued_lift, WheelMove::Commit(1));
        assert_eq!(
            pdf_wheel(
                queued_lift,
                coast.commit_token().unwrap(),
                1,
                6,
                75,
                width,
                0.0,
                true,
                false,
            ),
            PdfWheel::Busy { queued: 1 }
        );
        assert_eq!(coast.offer(down(-90.0), 6, 75), WheelMove::Follow(-90.0));
        assert_eq!(
            pdf_wheel(
                WheelMove::Follow(-90.0),
                0,
                1,
                6,
                75,
                width,
                0.0,
                true,
                false,
            ),
            PdfWheel::Pass
        );
        let replaced = coast.offer(up, 6, 75);
        assert_eq!(replaced, WheelMove::Commit(-1));
        assert_eq!(
            pdf_wheel(
                replaced,
                coast.commit_token().unwrap(),
                1,
                6,
                75,
                width,
                0.0,
                true,
                false,
            ),
            PdfWheel::Busy { queued: -1 }
        );
        glide.queued = -1;
        match glide_at(&glide, 80, 1) {
            GlideAt::Moving { page, .. } => assert_eq!(page, 3),
            other => panic!("a second finger moved the first page: {other:?}"),
        }
        match glide_at(&glide, 160, 1) {
            GlideAt::Landed { write, queued, .. } => assert_eq!((write, queued), (Some(4), -1)),
            other => panic!("the queue did not wait: {other:?}"),
        }
        let next = begin_slide(1, 75, 4, -1).unwrap();
        assert_eq!(pdf_turn(&next, 80, 1), PdfTurn::Moving { page: 4 });
        assert!(landed_progress(5, 0, 5).is_none());
        assert_eq!(store.resume().page, 5);
        let PdfTurn::Landed { page, queued } = pdf_turn(&next, 160, 1) else {
            panic!("the queued page did not land");
        };
        assert_eq!((page, queued), (3, 0));
        let (written, offset) = landed_progress(5, 0, page as u32 + 1).unwrap();
        assert_eq!((written, offset), (4, 0));
        crate::settings::record_progress(&file, book, written, Some(75), offset, "").unwrap();
        assert_eq!(store.resume().page, 4);

        assert_eq!(glide_at(&glide, 160, 9), GlideAt::Dropped);
        assert_eq!(store.resume().page, 4);

        let page_card = crate::form::page_frame(900.0, 800.0, true, false);
        let immersed = crate::form::page_frame(900.0, 800.0, true, true);
        assert!(!card_keeps_finger(
            page_card.card.w,
            page_card.card.h,
            immersed.card.w,
            immersed.card.h
        ));
        let narrow = crate::form::page_frame(817.0, 800.0, false, false);
        let wide = crate::form::page_frame(818.0, 800.0, true, false);
        assert!(!card_keeps_finger(
            narrow.card.w,
            narrow.card.h,
            wide.card.w,
            wide.card.h
        ));
        assert!(card_keeps_finger(
            page_card.card.w,
            page_card.card.h,
            page_card.card.w,
            page_card.card.h
        ));
        assert_eq!(store.resume().page, 4);

        let mut huge = Swipe::default();
        let over = huge.offer(down(900.0), 6, 75);
        match pdf_wheel(over, 0, 1, 6, 75, width, 0.0, false, false) {
            PdfWheel::Follow { shift } => assert_eq!(shift, width),
            other => panic!("the page slid past one page: {other:?}"),
        }
        assert_eq!(huge.offer(up, 6, 75), WheelMove::Commit(1));
        assert_eq!(store.resume().page, 4);

        let mut notch = Swipe::default();
        let wheel = notch.offer(
            WheelTick {
                finger: Finger::None,
                momentum: false,
                dx: 48.0,
                dy: 0.0,
            },
            6,
            75,
        );
        assert_eq!(wheel, WheelMove::Commit(1));
        match pdf_wheel(
            wheel,
            notch.commit_token().unwrap(),
            1,
            6,
            75,
            width,
            0.0,
            false,
            false,
        ) {
            PdfWheel::Begin(glide) => assert_eq!((glide.from, glide.to), (0.0, width)),
            other => panic!("a notch did not use the existing slide: {other:?}"),
        }
        assert_eq!(store.resume().page, 4);
        assert!(neighbor_on_the_right(80.0, false));

        let mut queued = Swipe::default();
        let wheel = |swipe: &mut Swipe, page: u32| {
            swipe.offer(
                WheelTick {
                    finger: Finger::None,
                    momentum: false,
                    dx: 48.0,
                    dy: 0.0,
                },
                page,
                75,
            )
        };
        assert_eq!(wheel(&mut queued, 4), WheelMove::Commit(1));
        let first = queued.commit_token().unwrap();
        assert_eq!(wheel(&mut queued, 4), WheelMove::Commit(1));
        let second = queued.commit_token().unwrap();
        assert!(queued.take_commit_post(first));
        queued.note_answer(second, "glide");
        assert_eq!(wheel(&mut queued, 4), WheelMove::Commit(1));
        let third = queued.commit_token().unwrap();
        assert_eq!(wheel(&mut queued, 4), WheelMove::Commit(1));
        let fourth = queued.commit_token().unwrap();
        assert_ne!(third, fourth);
        assert!(queued.take_commit_post(second));
        assert!(!queued.take_commit_post(third));
        assert_eq!(queued.commit_token(), Some(fourth));
        assert_eq!(store.resume().page, 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_book_drops_the_flying_token_and_saves_its_own_page() {
        let dir = std::env::temp_dir().join(format!("superreader-pdf-book-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let first_book = "/books/field.pdf";
        let second_book = "/books/other.pdf";
        crate::settings::record_open(&file, first_book, 4, Some(75), 1).unwrap();
        crate::settings::record_open(&file, second_book, 2, Some(40), 2).unwrap();
        let width = 400.0;
        let notch = |swipe: &mut Swipe, page: u32| {
            swipe.offer(
                WheelTick {
                    finger: Finger::None,
                    momentum: false,
                    dx: 48.0,
                    dy: 0.0,
                },
                page,
                75,
            )
        };
        let down = |dx: f64| WheelTick {
            finger: Finger::Down,
            momentum: false,
            dx,
            dy: 0.0,
        };
        let up = WheelTick {
            finger: Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };

        let mut swipe = Swipe::default();
        let mut book = 1u64;
        assert_eq!(notch(&mut swipe, 4), WheelMove::Commit(1));
        let old = swipe.commit_token().unwrap();
        let old_glide = match pdf_wheel(
            WheelMove::Commit(1),
            old,
            book,
            4,
            75,
            width,
            0.0,
            false,
            false,
        ) {
            PdfWheel::Begin(glide) => glide,
            other => panic!("the first book did not take the flying slot: {other:?}"),
        };
        let kept = crate::settings::SettingsBook {
            file: &file,
            book: first_book,
        };
        assert_eq!(kept.resume().page, 4);

        assert!(swipe.cancel_release());
        book = book.wrapping_add(1);
        assert_eq!(glide_at(&old_glide, 160, book), GlideAt::Dropped);
        assert!(!swipe.take_commit_post(old));
        assert_eq!(kept.resume().page, 4);

        assert_eq!(notch(&mut swipe, 2), WheelMove::Commit(1));
        let second = swipe.commit_token().unwrap();
        assert_ne!(second, old);
        let mut glide = match pdf_wheel(
            WheelMove::Commit(1),
            second,
            book,
            2,
            40,
            width,
            0.0,
            false,
            false,
        ) {
            PdfWheel::Begin(glide) => glide,
            other => panic!("the new book queued behind the old token: {other:?}"),
        };
        assert_eq!(notch(&mut swipe, 2), WheelMove::Commit(1));
        let third = swipe.commit_token().unwrap();
        assert_eq!(
            pdf_wheel(
                WheelMove::Commit(1),
                third,
                book,
                2,
                40,
                width,
                0.0,
                true,
                false
            ),
            PdfWheel::Busy { queued: 1 }
        );
        assert_ne!(third, second);
        glide.queued = 1;
        match glide_at(&glide, 80, book) {
            GlideAt::Moving { page, .. } => assert_eq!(page, 1),
            other => panic!("the new glide left its page early: {other:?}"),
        }
        let opened = crate::settings::SettingsBook {
            file: &file,
            book: second_book,
        };
        assert_eq!(opened.resume().page, 2);
        assert!(!swipe.take_commit_post(old));
        assert_eq!(opened.resume().page, 2);

        let GlideAt::Landed {
            write,
            token,
            queued,
            page,
        } = glide_at(&glide, 160, book)
        else {
            panic!("the new book's first glide did not land");
        };
        assert_eq!((page, write, queued, token), (2, Some(2), 1, second));
        assert!(swipe.take_commit_post(second));
        assert_eq!(swipe.commit_token(), Some(third));
        let (written, offset) = landed_progress(2, 0, page as u32 + 1).unwrap();
        assert_eq!((written, offset), (3, 0));
        crate::settings::record_progress(&file, second_book, written, Some(40), offset, "")
            .unwrap();
        assert_eq!(opened.resume().page, 3);
        assert_eq!(kept.resume().page, 4);
        assert!(!swipe.take_commit_post(old));
        assert_eq!(opened.resume().page, 3);

        swipe.note_answer(third, "glide");
        let mut queued_slide = begin_slide(book, 40, 2, 1).unwrap();
        assert_eq!(
            pdf_turn(&queued_slide, 80, book),
            PdfTurn::Moving { page: 2 }
        );
        assert_eq!(opened.resume().page, 3);
        assert_eq!(notch(&mut swipe, 3), WheelMove::Commit(1));
        let fourth = swipe.commit_token().unwrap();
        assert_ne!(fourth, third);
        queue_slide(&mut queued_slide, 1);
        let PdfTurn::Landed { page, queued } = pdf_turn(&queued_slide, 160, book) else {
            panic!("the queued page did not land");
        };
        assert_eq!((page, queued), (3, 1));
        assert!(swipe.take_commit_post(third));
        let (written, offset) = landed_progress(3, 0, page as u32 + 1).unwrap();
        assert_eq!((written, offset), (4, 0));
        crate::settings::record_progress(&file, second_book, written, Some(40), offset, "")
            .unwrap();
        assert_eq!(opened.resume().page, 4);
        assert!(!swipe.take_commit_post(old));
        assert_eq!(swipe.commit_token(), Some(fourth));
        assert_eq!(opened.resume().page, 4);
        assert_eq!(kept.resume().page, 4);

        let mut held = Swipe::default();
        assert_eq!(held.offer(down(80.0), 4, 40), WheelMove::Follow(80.0));
        assert!(held.cancel_release());
        assert_eq!(held.offer(up, 4, 40), WheelMove::Pass);
        assert_eq!(notch(&mut held, 4), WheelMove::Commit(1));
        let fresh = held.commit_token().unwrap();
        assert!(matches!(
            pdf_wheel(
                WheelMove::Commit(1),
                fresh,
                3,
                4,
                40,
                width,
                0.0,
                false,
                false
            ),
            PdfWheel::Begin(_)
        ));
        assert!(held.take_commit_post(fresh));
        crate::settings::record_progress(&file, second_book, 5, Some(40), 0, "").unwrap();
        assert_eq!(opened.resume().page, 5);
        let again = crate::settings::SettingsBook {
            file: &file,
            book: second_book,
        };
        assert_eq!(again.resume().page, 5);
        assert_eq!(kept.resume().page, 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_last_contents_choice_keeps_its_place_when_the_previous_document_posts() {
        let dir =
            std::env::temp_dir().join(format!("superreader-contents-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        let first = "/OEBPS/one.xhtml";
        let second = "/OEBPS/two.xhtml";
        let third = "/OEBPS/three.xhtml";
        let spine = [first.to_string(), second.to_string(), third.to_string()];
        let marks = crate::visit::contents_marks("Essay", &spine);
        crate::settings::record_progress(&file, book, 1, None, 0, first).unwrap();
        let mut reader = Reader::default();
        reader.place = ReadingPlace {
            href: first.to_string(),
            page: 1,
            offset: 0,
        };
        claim_loaded_document(&mut reader, 1);
        let mut store = crate::settings::SettingsBook { file: &file, book };
        let earlier = handle_chapter(&mut reader, 1, &marks, &spine, first, &mut store);
        assert!(matches!(
            earlier.go,
            Some(ChapterMove::Chapter {
                index: 1,
                offset: 0
            })
        ));
        claim_loaded_document(&mut reader, 2);
        let last = handle_chapter(&mut reader, 2, &marks, &spine, second, &mut store);
        assert!(matches!(
            last.go,
            Some(ChapterMove::Chapter {
                index: 2,
                offset: 0
            })
        ));
        claim_loaded_document(&mut reader, 3);
        assert_eq!(reader.place.href, third);
        assert_eq!(reader.place.page, 1);
        assert_eq!(store.resume().page, 1);
        assert_eq!(store.resume().href, third);

        let stale = serde_json::json!({
            "kind": "turn",
            "doc": 2,
            "load": 2,
            "page": 8,
            "pages": 12,
            "sentence": 400,
            "frames": []
        });
        let ignored = handle_page(&mut reader, &stale, third, &mut store);
        assert!(!ignored.accepted && !ignored.wrote);
        let same_doc_old_epoch = serde_json::json!({
            "kind": "turn",
            "doc": reader.doc,
            "load": reader.restore_load - 1,
            "page": 8,
            "pages": 12,
            "sentence": 400,
            "frames": []
        });
        let aged = handle_page(&mut reader, &same_doc_old_epoch, third, &mut store);
        assert!(!aged.accepted && !aged.wrote);
        assert_eq!(reader.place.href, third);
        assert_eq!(reader.place.page, 1);
        assert_eq!(reader.place.offset, 0);
        assert_eq!(store.resume().href, third);
        assert_eq!(store.resume().page, 1);
        assert_eq!(store.resume().offset, 0);

        let current = serde_json::json!({
            "kind": "turn",
            "doc": reader.doc,
            "load": reader.restore_load,
            "page": 2,
            "pages": 12,
            "sentence": 20,
            "frames": []
        });
        let written = handle_page(&mut reader, &current, third, &mut store);
        assert!(written.accepted && written.wrote);
        assert_eq!(reader.place.href, third);
        assert_eq!(reader.doc, 3);
        assert_eq!(store.resume().href, third);
        assert_eq!(store.resume().page, 2);
        assert_eq!(store.resume().offset, 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_same_chapter_choice_keeps_its_sentence_when_the_old_turn_arrives() {
        let dir =
            std::env::temp_dir().join(format!("superreader-same-chapter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        let chapter = "/OEBPS/chapter.xhtml";
        let spine = [chapter.to_string()];
        let marks = [
            ChapterMark {
                title: "Middle".to_string(),
                href: chapter.to_string(),
                offset: 400,
                page: 0,
            },
            ChapterMark {
                title: "Start".to_string(),
                href: chapter.to_string(),
                offset: 0,
                page: 0,
            },
        ];
        crate::settings::record_progress(&file, book, 4, None, 800, chapter).unwrap();
        let mut reader = Reader::default();
        reader.doc = 1;
        reader.restore_load = 1;
        reader.place = ReadingPlace {
            href: chapter.to_string(),
            page: 4,
            offset: 800,
        };
        let mut store = crate::settings::SettingsBook { file: &file, book };
        let first = handle_chapter(&mut reader, 0, &marks, &spine, chapter, &mut store);
        assert_eq!(first.go, Some(ChapterMove::Reveal { offset: 400 }));
        let chosen = sentence_in_open_document(&mut reader, 400, SentenceCause::Chosen);
        assert_eq!(chosen.load, Some(2));
        let laid = sentence_in_open_document(&mut reader, 400, SentenceCause::Held);
        assert_eq!(laid.load, None);
        assert_eq!(reader.restore_load, 2);
        let kept = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": 2,
            "page": 5,
            "pages": 12,
            "sentence": 12,
            "frames": []
        });
        let laid_out = handle_page(&mut reader, &kept, chapter, &mut store);
        assert!(laid_out.accepted && laid_out.wrote);
        assert_eq!(store.resume().page, 5);
        assert_eq!(store.resume().offset, 12);

        let link = serde_json::json!({
            "kind": "link",
            "raw": "#note",
            "load": reader.restore_load,
            "measured": true,
            "offset": 80
        });
        let linked = handle_link_message(&mut reader, &link, chapter, &spine, &mut store);
        assert_eq!(linked.go, Some(ChapterMove::Reveal { offset: 80 }));
        let after_link = sentence_in_open_document(&mut reader, 80, SentenceCause::Chosen);
        assert_eq!(after_link.load, Some(3));
        let jumped = place_jump(
            &reader.place,
            &ReadingPlace {
                href: chapter.to_string(),
                page: reader.place.page,
                offset: 0,
            },
            &spine,
            None,
        );
        assert_eq!(jumped.go, ChapterMove::Reveal { offset: 0 });
        let after_jump = sentence_in_open_document(&mut reader, 0, SentenceCause::Chosen);
        assert_eq!(after_jump.load, Some(4));
        assert_eq!(store.resume().href, chapter);
        assert_eq!(store.resume().offset, 80);

        let issued = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": 1,
            "page": 8,
            "pages": 12,
            "sentence": 400,
            "frames": []
        });
        let ignored = handle_page(&mut reader, &issued, chapter, &mut store);
        assert!(!ignored.accepted && !ignored.wrote);
        let between = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": 3,
            "page": 6,
            "pages": 12,
            "sentence": 10,
            "frames": []
        });
        let aged = handle_page(&mut reader, &between, chapter, &mut store);
        assert!(!aged.accepted && !aged.wrote);
        assert_eq!(store.resume().page, 5);
        assert_eq!(store.resume().offset, 80);

        let fresh = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": reader.restore_load,
            "page": 2,
            "pages": 12,
            "sentence": 20,
            "frames": []
        });
        let written = handle_page(&mut reader, &fresh, chapter, &mut store);
        assert!(written.accepted && written.wrote);
        assert_eq!(store.resume().href, chapter);
        assert_eq!(store.resume().page, 2);
        assert_eq!(store.resume().offset, 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_dismissed_book_does_not_write_the_book_now_open() {
        let dir =
            std::env::temp_dir().join(format!("superreader-reopen-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let left = "/books/left.epub";
        let opened = "/books/opened.epub";
        let same = "/books/left.epub";
        let left_href = "/OEBPS/left.xhtml";
        let opened_href = "/OEBPS/opened.xhtml";
        crate::settings::record_progress(&file, left, 4, None, 80, left_href).unwrap();
        crate::settings::record_progress(&file, opened, 1, None, 0, opened_href).unwrap();
        let mut reader = Reader::default();
        claim_loaded_document(&mut reader, 1);
        open_place(&mut reader, 4, 80, left_href, true);
        assert_eq!(reader.restore_load, 1);
        leave(&mut reader);
        claim_loaded_document(&mut reader, 1);
        open_place(&mut reader, 1, 0, opened_href, false);
        assert_eq!(reader.doc, 1);
        assert_eq!(reader.restore_load, 2);
        let mut opened_store = crate::settings::SettingsBook {
            file: &file,
            book: opened,
        };
        let stale = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": 1,
            "page": 9,
            "pages": 12,
            "sentence": 50,
            "frames": []
        });
        let ignored = handle_page(&mut reader, &stale, opened_href, &mut opened_store);
        assert!(!ignored.accepted && !ignored.wrote);
        assert_eq!(opened_store.resume().page, 1);
        assert_eq!(opened_store.resume().offset, 0);
        assert_eq!(opened_store.resume().href, opened_href);
        let left_store = crate::settings::SettingsBook {
            file: &file,
            book: left,
        };
        assert_eq!(left_store.resume().page, 4);
        assert_eq!(left_store.resume().offset, 80);
        assert_eq!(left_store.resume().href, left_href);

        let own = serde_json::json!({
            "kind": "turn",
            "doc": reader.doc,
            "load": reader.restore_load,
            "page": 2,
            "pages": 12,
            "sentence": 15,
            "frames": []
        });
        let written = handle_page(&mut reader, &own, opened_href, &mut opened_store);
        assert!(written.accepted && written.wrote);
        assert_eq!(opened_store.resume().href, opened_href);
        assert_eq!(opened_store.resume().page, 2);
        assert_eq!(opened_store.resume().offset, 15);
        assert_eq!(left_store.resume().page, 4);
        assert_eq!(left_store.resume().offset, 80);

        leave(&mut reader);
        claim_loaded_document(&mut reader, 1);
        open_place(&mut reader, 4, 80, left_href, true);
        let mut same_store = crate::settings::SettingsBook {
            file: &file,
            book: same,
        };
        let late_same = serde_json::json!({
            "kind": "turn",
            "doc": 1,
            "load": 1,
            "page": 9,
            "pages": 12,
            "sentence": 50,
            "frames": []
        });
        let kept = handle_page(&mut reader, &late_same, left_href, &mut same_store);
        assert!(!kept.accepted && !kept.wrote);
        assert_eq!(same_store.resume().href, left_href);
        assert_eq!(same_store.resume().page, 4);
        assert_eq!(same_store.resume().offset, 80);
        let resumed = serde_json::json!({
            "kind": "turn",
            "doc": reader.doc,
            "load": reader.restore_load,
            "page": 3,
            "pages": 12,
            "sentence": 12,
            "frames": []
        });
        let moved = handle_page(&mut reader, &resumed, left_href, &mut same_store);
        assert!(moved.accepted && moved.wrote);
        assert_eq!(same_store.resume().href, left_href);
        assert_eq!(same_store.resume().page, 3);
        assert_eq!(same_store.resume().offset, 12);
        assert_eq!(opened_store.resume().page, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_last_pdf_contents_page_retires_the_glide_and_is_the_saved_page() {
        let stay = ChapterMove::Stay;
        let page = ChapterMove::Page { page: 9 };
        let chapter = ChapterMove::Chapter {
            index: 2,
            offset: 0,
        };
        let reveal = ChapterMove::Reveal { offset: 40 };
        assert!(contents_choice_retires_pdf_motion(&stay));
        assert!(contents_choice_retires_pdf_motion(&page));
        assert!(contents_choice_retires_pdf_motion(&chapter));
        assert!(contents_choice_retires_pdf_motion(&reveal));

        let flying = FingerGlide {
            book: 1,
            page: 4,
            from: 0.0,
            to: 400.0,
            incoming: Some(5),
            write: Some(4),
            queued: 0,
            token: 7,
        };
        assert_eq!(
            glide_at(&flying, 160, 1),
            GlideAt::Landed {
                page: 4,
                write: Some(4),
                queued: 0,
                token: 7,
            }
        );
        let dir =
            std::env::temp_dir().join(format!("superreader-pdf-contents-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/fitted.pdf";
        crate::settings::record_progress(&file, book, 1, Some(12), 0, "").unwrap();
        let mut reader = Reader::default();
        reader.place.page = 1;
        let marks = [
            ChapterMark {
                title: "Three".to_string(),
                href: String::new(),
                offset: 0,
                page: 3,
            },
            ChapterMark {
                title: "Nine".to_string(),
                href: String::new(),
                offset: 0,
                page: 9,
            },
        ];
        let mut store = crate::settings::SettingsBook { file: &file, book };
        let first = handle_chapter(&mut reader, 0, &marks, &[], "", &mut store);
        let last = handle_chapter(&mut reader, 1, &marks, &[], "", &mut store);
        assert_eq!(first.go, Some(ChapterMove::Page { page: 3 }));
        assert_eq!(last.go, Some(ChapterMove::Page { page: 9 }));
        let mut glide = Some(flying);
        let mut motion = Some(7u32);
        let mut drag = 12.0;
        let mut saved_page = 1u32;
        let mut saved_offset = 0u32;
        assert!(apply_contents_choice(
            &mut glide,
            &mut motion,
            &mut drag,
            &mut saved_page,
            &mut saved_offset,
            &first.go.unwrap(),
            &mut store,
        ));
        assert!(glide.is_none());
        assert!(motion.is_none());
        assert_eq!(drag, 0.0);
        assert_eq!(store.resume().page, 3);
        assert_eq!((saved_page, saved_offset), (3, 0));
        glide = Some(flying);
        motion = Some(7);
        drag = 4.0;
        assert!(apply_contents_choice(
            &mut glide,
            &mut motion,
            &mut drag,
            &mut saved_page,
            &mut saved_offset,
            &last.go.unwrap(),
            &mut store,
        ));
        assert!(glide.is_none());
        assert_eq!(store.resume().page, 9);
        assert_eq!(store.resume().offset, 0);
        assert_eq!((saved_page, saved_offset), (9, 0));
        assert_eq!(flying.write, Some(4));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pdf_goto_click_lands_on_the_page_and_keeps_one_return() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../pdf/fixtures/goto-links.pdf"
        );
        let bytes = std::fs::read(path).unwrap();
        let (doc, _, _) = spike_pdf::Document::open(
            spike_report::PdfEngine::Pdfium,
            bytes,
            &spike_report::pdfium_lib(),
        )
        .unwrap();
        let links = doc.page_links(0);
        let goto = links
            .iter()
            .find(|link| link.page == Some(2))
            .expect("cross-page GoTo");
        assert!((goto.left - 20.0).abs() < 0.2 && (goto.top - 290.0).abs() < 0.2);
        let view = |width: f64, height: f64, px: f32, py: f32| {
            let x = 100.0 + f64::from(px) / 200.0 * width;
            let y = 40.0 + (300.0 - f64::from(py)) / 300.0 * height;
            (x, y)
        };
        let (x, y) = view(100.0, 150.0, goto.right, goto.top);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0, &links, 1),
            Some((1, spike_pdf::LinkHit::Page(2)))
        );
        let (x, y) = view(400.0, 600.0, 55.0, 275.0);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 400.0, 600.0, 200.0, 300.0, &links, 1)
                .map(|(_, hit)| hit),
            Some(spike_pdf::LinkHit::Page(2))
        );
        assert_eq!(
            pdf_user_point(99.9, 40.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0),
            None
        );
        let top = pdf_user_point(200.0, 40.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0).unwrap();
        assert!((top.0 - 200.0).abs() < 0.01 && (top.1 - 300.0).abs() < 0.01);
        let bottom = pdf_user_point(100.0, 190.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0).unwrap();
        assert!(bottom.0.abs() < 0.01 && bottom.1.abs() < 0.01);
        assert_eq!(
            spike_pdf::hit_page_link(&links, goto.right + 0.2, goto.top),
            None
        );
        assert!(margin_owns_point(
            10.0,
            10.0,
            true,
            Some((0.0, 0.0, 40.0, 40.0))
        ));
        assert!(!margin_owns_point(
            10.0,
            10.0,
            false,
            Some((0.0, 0.0, 40.0, 40.0))
        ));

        let first = pdf_press(
            Some(spike_pdf::LinkHit::Page(2)),
            1,
            2,
            None,
            PageEdge::Forward,
        );
        assert_eq!(
            first,
            PdfPress::Follow {
                page: 2,
                back: Some(1)
            }
        );
        let second = pdf_press(
            Some(spike_pdf::LinkHit::Page(1)),
            2,
            2,
            Some(1),
            PageEdge::Back,
        );
        assert_eq!(
            second,
            PdfPress::Follow {
                page: 1,
                back: Some(1)
            }
        );
        let third = pdf_press(
            Some(spike_pdf::LinkHit::Page(2)),
            1,
            2,
            Some(1),
            PageEdge::None,
        );
        assert_eq!(
            third,
            PdfPress::Follow {
                page: 2,
                back: Some(1)
            }
        );
        assert_eq!(
            pdf_press(
                Some(spike_pdf::LinkHit::Held),
                1,
                2,
                None,
                PageEdge::Forward
            ),
            PdfPress::Stay
        );
        assert_eq!(
            pdf_press(
                Some(spike_pdf::LinkHit::Page(1)),
                1,
                2,
                None,
                PageEdge::Back
            ),
            PdfPress::Stay
        );
        assert_eq!(
            pdf_press(None, 1, 2, Some(1), PageEdge::Forward),
            PdfPress::Turn(PageEdge::Forward)
        );
        assert_eq!(link_back_for_book(Some(1), true), Some(1));
        assert_eq!(link_back_for_book(Some(1), false), None);

        let dir = std::env::temp_dir().join(format!("superreader-pdf-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/goto-links.pdf";
        crate::settings::record_progress(&file, book, 1, None, 0, "").unwrap();
        let mut store = crate::settings::SettingsBook { file: &file, book };
        let flying = FingerGlide {
            book: 1,
            page: 1,
            from: 40.0,
            to: 400.0,
            incoming: Some(2),
            write: Some(1),
            queued: 0,
            token: 3,
        };
        let mut glide = Some(flying);
        let mut motion = Some(9u32);
        let mut drag = 18.0;
        let mut saved_page = 1u32;
        let mut saved_offset = 4u32;
        let PdfPress::Follow { page, back } = first else {
            panic!("cross-page");
        };
        assert_eq!(back, Some(1));
        assert!(apply_contents_choice(
            &mut glide,
            &mut motion,
            &mut drag,
            &mut saved_page,
            &mut saved_offset,
            &ChapterMove::Page { page },
            &mut store,
        ));
        assert!(glide.is_none());
        assert!(motion.is_none());
        assert_eq!(drag, 0.0);
        assert_eq!(store.resume().page, 2);
        assert_eq!((saved_page, saved_offset), (2, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
