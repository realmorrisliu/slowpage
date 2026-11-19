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

/// The place that belongs to the href on screen. A jump aimed at that href
/// wins. A saved offset from a different href does not.
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
    if saved_href == shown_href {
        return ReadingPlace {
            href: shown_href.to_string(),
            page: saved_page.max(1),
            offset: awaiting.unwrap_or(saved_offset),
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
    let step = if nav.pick < 0 {
        chapter_return(&here, nav.back, nav.spine)
    } else {
        chapter_jump(&here, nav.marks, nav.pick as usize, nav.spine, nav.back)
    };
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
                if saved_href != *href || saved_page != 1 || saved_offset != offset {
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

    pub fn offer(&mut self, tick: WheelTick, page: u32, pages: u32) -> WheelMove {
        if self.ignore {
            if matches!(tick.finger, Finger::Up | Finger::Cancelled) && !tick.momentum {
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
            return if matches!(tick.finger, Finger::Up | Finger::Cancelled) && !tick.momentum {
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
                WheelMove::Cancel
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
    /// A caret read is in flight. The poll does not chase and does not end
    /// the walk until the hit or the miss is applied.
    pub anchoring: bool,
    /// The sample this resize asked for. A post from an older sample is not
    /// the current character.
    pub probe: u64,
    /// The saved page number is not a chase. A miss, a timeout, or a failed
    /// anchor write sets it. The frame may still move, and a later size may
    /// measure. It does not freeze the draw.
    pub hold_layout: bool,
    /// The next restore answers a user post this reader refused or accepted.
    /// The page blocks an older restore until that answer arrives.
    pub restore_ack: bool,
    /// The loaded chapter document. A restore or a drop from an older document
    /// does not run. A user key does not carry this.
    pub restore_load: u64,
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
            anchoring: false,
            probe: 0,
            hold_layout: false,
            restore_ack: false,
            restore_load: 0,
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

fn clear_resume(reader: &mut Reader) {
    reader.jump_href.clear();
    reader.jump_offset = None;
    reader.awaiting = None;
    reader.pending = 0;
    reader.hold = None;
    reader.reveal = None;
    reader.walk = None;
    reader.measured = None;
    reader.anchoring = false;
    reader.hold_layout = false;
}

/// Accept a script message. The place changes only after `write_place`
/// returns true. A refused token, including one past the floor, changes nothing.
pub fn handle_page(
    reader: &mut Reader,
    value: &serde_json::Value,
    href: &str,
    store: &mut dyn PlaceFile,
) -> Apply {
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
/// This does not read the inbox.
pub fn handle_chapter(
    reader: &mut Reader,
    pick: isize,
    marks: &[ChapterMark],
    spine: &[String],
    href: &str,
    store: &mut dyn PlaceFile,
) -> Apply {
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
    reader.anchoring = false;
    reader.hold_layout = false;
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
    reader.swipe.cancel_release()
}

pub fn reset_restore_epoch(reader: &mut Reader) {
    reader.restore_load = reader.restore_load.saturating_add(1);
    reader.restore_ack = false;
    reader.measured = None;
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

/// The draw `place_page` applies. A failed sample moves the box and does not
/// seek the saved page. The next size can measure. A matching box does not
/// cancel and does not schedule a chase.
pub fn layout_frame(reader: &mut Reader, tick: FrameTick) -> DrawPlan {
    if tick.awaiting {
        let hold = current_hold(tick.hold, tick.hold_probe, reader.probe);
        let missed = tick.missed && hold.is_none();
        let timed_out = tick.timed_out && hold.is_none();
        match finish_hold_offset(hold, missed, timed_out) {
            HoldFinish::Wait => return plan(Draw::Rest, false, true),
            HoldFinish::Miss => {
                reader.probe = reader.probe.saturating_add(1);
                reader.anchoring = false;
                reader.hold_layout = true;
                return plan(Draw::Move { reveal: None }, false, false);
            }
            HoldFinish::Reveal(offset) => {
                reader.anchoring = true;
                reader.hold_layout = false;
                return plan(
                    Draw::Move {
                        reveal: Some(offset),
                    },
                    false,
                    false,
                );
            }
        }
    }
    let chase = chase_due(reader, tick);
    match place_step(tick.fitted_same, tick.zoom_same, tick.stage_same) {
        PlaceStep::Hold => plan(Draw::Rest, false, chase),
        PlaceStep::Cancel { reflow: false } => plan(Draw::Rest, true, chase),
        PlaceStep::Cancel { reflow: true } => {
            if tick.settled && tick.reflowable {
                if let Some(offset) = reader.measured {
                    reader.anchoring = false;
                    return plan(
                        Draw::Move {
                            reveal: Some(offset),
                        },
                        false,
                        false,
                    );
                }
                if reader.anchoring {
                    return plan(Draw::Rest, false, true);
                }
                reader.probe = reader.probe.saturating_add(1);
                reader.anchoring = true;
                let page = reader.walk.unwrap_or(0);
                let probe = reader.probe;
                return plan(Draw::Measure { page, probe }, true, true);
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
    Reveal(u32),
}

pub fn finish_hold_offset(hold: Option<u32>, missed: bool, timed_out: bool) -> HoldFinish {
    if let Some(ch) = hold {
        return HoldFinish::Reveal(ch);
    }
    if missed || timed_out {
        HoldFinish::Miss
    } else {
        HoldFinish::Wait
    }
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
        assert_eq!(swipe.offer(up, 1, 10), WheelMove::Cancel);
        assert_eq!(swipe.offer(down(80.0), 10, 10), WheelMove::Follow(0.0));
        assert_eq!(swipe.offer(up, 10, 10), WheelMove::Cancel);

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
        let failed = layout_frame(&mut reader, tick);
        assert_eq!(failed.draw, Draw::Move { reveal: None });
        assert!(!failed.cancel);
        assert!(!failed.pump);
        assert!(reader.hold_layout);
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
            let Draw::Measure {
                page: 10,
                probe: next,
            } = again.draw
            else {
                panic!("the next size measures, {again:?}");
            };
            assert!(again.cancel);
            tick.awaiting = true;
            tick.hold_probe = next;
            tick.missed = true;
            let missed = layout_frame(&mut reader, tick);
            assert_eq!(missed.draw, Draw::Move { reveal: None });
            assert!(!missed.cancel);
            assert!(!missed.pump);
            tick.awaiting = false;
            tick.missed = false;
            tick.fitted_same = true;
            let rested = layout_frame(&mut reader, tick);
            assert!(!rested.cancel && !rested.pump, "{rested:?}");
        }
        assert_eq!(reader.walk, Some(10));
        reader.measured = Some(42);
        reader.anchoring = false;
        tick.fitted_same = false;
        let known = layout_frame(&mut reader, tick);
        assert_eq!(known.draw, Draw::Move { reveal: Some(42) });
        assert!(!known.cancel);
        assert!(!known.pump);
        assert_eq!(reader.walk, Some(10));
        reader.measured = Some(0);
        let zero = layout_frame(&mut reader, tick);
        assert_eq!(zero.draw, Draw::Move { reveal: Some(0) });
        assert!(!zero.pump);
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
}
