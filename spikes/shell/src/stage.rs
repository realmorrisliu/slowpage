use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::rc::Rc;
use std::time::Instant;

use nsh_platform_macos::objc2::rc::{Allocated, Retained};
use nsh_platform_macos::objc2::runtime::{AnyClass, AnyObject, Sel};
use nsh_platform_macos::objc2::{
    define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use nsh_platform_macos::objc2_app_kit::{NSEvent, NSResponder, NSView, NSWindow, NSWindowButton};
use nsh_platform_macos::objc2_foundation::{
    NSArray, NSObjectProtocol, NSPoint, NSRect, NSRunLoopCommonModes, NSSize, NSString,
};
use nsh_platform_macos::{ComponentContext, Components, NativeComponent};
use objc2_core_graphics::CGContext;
use serde_json::{json, Value};
use spike_pdf::{
    paint_find, paint_highlight, select_points, Document, FindMark, PageRead, PageText, Rgba,
};
use spike_report::{Anchor, PdfEngine, PixelScale};

use crate::bitmap;

struct Ivars {
    state: Rc<RefCell<State>>,
}

define_class!(
    #[unsafe(super(NSView, NSResponder, objc2::runtime::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct StageView;

    unsafe impl NSObjectProtocol for StageView {}

    impl StageView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(pumpChrome))]
        fn pump_chrome(&self) {
            let _: () = unsafe { msg_send![self, setNeedsDisplay: true] };
        }

        #[unsafe(method(showContents:))]
        fn show_contents(&self, _sender: Option<&AnyObject>) {
            let titles = {
                let state = self.ivars().state.borrow();
                let titles = state
                    .web
                    .as_ref()
                    .map(|web| {
                        crate::visit::contents_marks(&web.book_title, &web.spine())
                            .into_iter()
                            .map(|mark| mark.title)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if titles.is_empty() && !crate::web::reader_has_back() {
                    return;
                }
                titles
            };
            pop_chapters(self, &titles, crate::web::reader_has_back());
        }

        #[unsafe(method(pickChapter:))]
        fn pick_chapter(&self, sender: Option<&AnyObject>) {
            let Some(sender) = sender else {
                return;
            };
            let tag: isize = unsafe { msg_send![sender, tag] };
            self.ivars().state.borrow_mut().apply_chapter(tag);
            let _: () = unsafe { msg_send![self, setNeedsDisplay: true] };
        }

        #[unsafe(method(pumpLights))]
        fn pump_lights(&self) {
            let window: Option<Retained<NSWindow>> = unsafe { msg_send![self, window] };
            let live = window.as_ref().is_some_and(|window| window.inLiveResize());
            if let Some(window) = window {
                let queued = crate::launch::take_opens();
                if !queued.is_empty() {
                    let mut state = self.ivars().state.borrow_mut();
                    for path in queued {
                        state.open_requested(self, &path);
                    }
                }
                let (chrome, page, before, open) = {
                    let state = self.ivars().state.borrow();
                    (
                        state.chrome.clone(),
                        state.page_box(),
                        (
                            state.chrome.bar_w.get(),
                            state.chrome.bar_gen.get(),
                            state.chrome.bar_dirty.get(),
                        ),
                        state.web.is_some(),
                    )
                };
                chrome.reading.set(crate::reading::reader_shown(
                    open,
                    crate::visit::on_library(),
                ));
                crate::chrome::sync(&window, &chrome, page);
                if crate::chrome::shell_on()
                    && chrome.reading.get()
                    && chrome.lights_changed(chrome.lights.get())
                {
                    self.ivars().state.borrow_mut().publish();
                }
                if open {
                    let target = unsafe { &*(std::ptr::from_ref(self).cast::<AnyObject>()) };
                    crate::chrome::bind_contents(&chrome, target);
                }
                let after = (chrome.bar_w.get(), chrome.bar_gen.get(), chrome.bar_dirty.get());
                if before != after {
                    let _: () = unsafe { msg_send![self, setNeedsDisplay: true] };
                }
                if crate::chrome::shell_on() {
                    if let Some((width, height)) = crate::chrome::content_size(&window) {
                        publish_shell_if_resized(
                            &self.ivars().state,
                            width,
                            height,
                            "pump",
                        );
                    }
                }
                self.ivars().state.borrow().note_probe(self);
                let progressed = self.ivars().state.borrow_mut().pump_find();
                if progressed {
                    self.ivars().state.borrow_mut().publish();
                    let _: () = unsafe { msg_send![self, setNeedsDisplay: true] };
                }
            }
            // afterDelay alone is NSDefaultRunLoopMode. A drag tracks in
            // NSEventTrackingRunLoopMode, so that timer does not run until
            // the mouse is released and the layout jumps once.
            schedule_shell_pump(self, if live { 1.0 / 60.0 } else { 0.05 });
        }

        #[unsafe(method(pollSettings))]
        fn poll_settings(&self) {
            let changed = self.ivars().state.borrow_mut().reload_trim_from_file();
            if changed {
                let _: () = unsafe { msg_send![self, setNeedsDisplay: true] };
            }
            let _: () = unsafe {
                msg_send![
                    self,
                    performSelector: sel!(pollSettings),
                    withObject: std::ptr::null::<objc2::runtime::AnyObject>(),
                    afterDelay: 0.4_f64
                ]
            };
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let state = self.ivars().state.clone();
            state.borrow_mut().draw(self);
        }

        #[unsafe(method(layout))]
        fn layout_view(&self) {
            let _: () = unsafe { msg_send![super(self), layout] };
            let bounds: NSRect = unsafe { msg_send![self, bounds] };
            let window: Option<Retained<NSWindow>> = unsafe { msg_send![self, window] };
            let view: &NSView = unsafe { &*std::ptr::from_ref(self).cast::<NSView>() };
            crate::chrome::write_bounds(view);
            let fullscreen = window.as_ref().is_some_and(|window| {
                window.styleMask().contains(objc2_app_kit::NSWindowStyleMask::FullScreen)
            });
            let (chrome, page) = {
                let mut state = self.ivars().state.borrow_mut();
                if state.chrome.full.get() != fullscreen {
                    state.chrome.mark_bar_dirty();
                }
                state.chrome.full.set(fullscreen);
                state.stage_w = bounds.size.width;
                state.stage_h = bounds.size.height;
                state.chrome.stage_h.set(bounds.size.height);
                state.chrome.laying_out.set(true);
                state.place_page();
                (state.chrome.clone(), state.page_box())
            };
            if let Some(window) = window {
                crate::chrome::sync(&window, &chrome, page);
                // AppKit calls layout during the drag. The pump timer does
                // not. Publish the content view's size from this callback so
                // the shelf follows the window instead of waiting for mouse-up.
                if crate::chrome::shell_on() {
                    if let Some((width, height)) = crate::chrome::content_size(&window) {
                        publish_shell_if_resized(
                            &self.ivars().state,
                            width,
                            height,
                            "layout",
                        );
                    }
                }
            }
            chrome.laying_out.set(false);
        }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            if crate::visit::wheel_scrolls_shelf(crate::chrome::shell_on()) {
                // Covers forward a wheel here. The shelf is the NSH scroll
                // above this stage, so move that instead of the PDF offset.
                let mut dy: f64 = unsafe { msg_send![event, scrollingDeltaY] };
                if dy.abs() < 0.01 {
                    let lines: f64 = unsafe { msg_send![event, deltaY] };
                    dy = lines * 16.0;
                }
                crate::form::nudge_shelf(-dy as f32);
                return;
            }
            let precise: bool = unsafe { msg_send![event, hasPreciseScrollingDeltas] };
            let scroll_x: f64 = unsafe { msg_send![event, scrollingDeltaX] };
            let scroll_y: f64 = unsafe { msg_send![event, scrollingDeltaY] };
            let line_x: f64 = unsafe { msg_send![event, deltaX] };
            let line_y: f64 = unsafe { msg_send![event, deltaY] };
            let phase = event.phase().0 as u64;
            let momentum = event.momentumPhase().0 as u64;
            let (x, y) = point_in(self, event);
            let mut state = self.ivars().state.borrow_mut();
            let width = if state.shell_w > 2.0 {
                state.shell_w
            } else {
                state.stage_w
            };
            let height = if state.shell_h > 2.0 {
                state.shell_h
            } else {
                state.stage_h
            };
            let presented = crate::ease::present(
                width,
                height,
                state.chrome.content_top.get(),
                crate::ease::current(),
            );
            if crate::visit::wheel_step(
                crate::form::margin_claims_wheel(x, y, presented.margin),
                false,
            ) != crate::visit::WheelStep::Page
            {
                return;
            }
            if state.fits_card() && matches!(state.mode, Mode::Pdfium | Mode::Hayro) {
                state.wheel_pdf(
                    phase,
                    momentum,
                    crate::visit::wheel_points(precise, scroll_x, line_x),
                    crate::visit::wheel_points(precise, scroll_y, line_y),
                );
            } else {
                state.scroll(scroll_y);
            }
            drop(state);
            unsafe { msg_send![self, setNeedsDisplay: true] }
        }

        #[unsafe(method(magnifyWithEvent:))]
        fn magnify(&self, event: &NSEvent) {
            let mag: f64 = unsafe { msg_send![event, magnification] };
            self.ivars().state.borrow_mut().pinch(mag);
            unsafe { msg_send![self, setNeedsDisplay: true] }
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let point = point_in(self, event);
            let placed = self
                .ivars()
                .state
                .borrow()
                .placed
                .as_ref()
                .map(|item| (item.0, item.1, item.2, item.3));
            if crate::chrome::should_drag_margin(point.0, point.1, placed) {
                let window: Option<Retained<NSWindow>> = unsafe { msg_send![self, window] };
                if let Some(window) = window {
                    let _: () = unsafe { msg_send![&*window, performWindowDragWithEvent: event] };
                }
                return;
            }
            {
                let mut state = self.ivars().state.borrow_mut();
                state.drag_gen = state.book_gen;
                state.drag = Some((point, point));
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let point = point_in(self, event);
            if let Some((start, _)) = self.ivars().state.borrow().drag {
                self.ivars().state.borrow_mut().drag = Some((start, point));
            }
            unsafe { msg_send![self, setNeedsDisplay: true] }
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let point = point_in(self, event);
            let bounds: NSRect = unsafe { msg_send![self, bounds] };
            let state = self.ivars().state.clone();
            let mut state = state.borrow_mut();
            if let Some((start, _)) = state.drag.take() {
                let click = !crate::visit::click_is_drag(point.0 - start.0, point.1 - start.1);
                let turns = crate::ease::pointer_turns(crate::ease::current());
                let on_page = click
                    && turns
                    && !crate::visit::wheel_scrolls_shelf(crate::chrome::shell_on());
                let edge = if on_page {
                    state.page_edge_at(point.0, point.1, bounds.size.width, bounds.size.height)
                } else {
                    crate::visit::PageEdge::None
                };
                let found = if on_page {
                    state.pdf_link_target(point.0, point.1, bounds.size.width, bounds.size.height)
                } else {
                    None
                };
                let here = found.map(|(page, _)| page).unwrap_or(1);
                let hit = found.map(|(_, hit)| hit);
                // Open loads at most 80 page sizes. A link may name a later page.
                let pages = state
                    .pdf
                    .as_ref()
                    .map(|pdf| pdf.doc.page_count() as u32)
                    .unwrap_or(0);
                match crate::visit::pdf_press(hit, here, pages, edge) {
                    crate::visit::PdfPress::Follow { page } => {
                        crate::web::record_pdf_goto(here, page);
                        let _ = state.show_pdf_page(page);
                        state.publish();
                    }
                    crate::visit::PdfPress::Stay => {}
                    crate::visit::PdfPress::Turn(crate::visit::PageEdge::Back) => {
                        state.command("page", &json!({"delta": -1, "intent": "edge"}))
                    }
                    crate::visit::PdfPress::Turn(crate::visit::PageEdge::Forward) => {
                        state.command("page", &json!({"delta": 1, "intent": "edge"}))
                    }
                    crate::visit::PdfPress::Turn(crate::visit::PageEdge::None) => {
                        state.finish_drag(start, point)
                    }
                }
            }
            drop(state);
            unsafe { msg_send![self, setNeedsDisplay: true] }
        }
    }
);

fn opened_at_ms() -> i64 {
    nsh_core::time::unix_ms() as i64
}

fn point_in(view: &StageView, event: &NSEvent) -> (f64, f64) {
    let window_point: NSPoint = unsafe { msg_send![event, locationInWindow] };
    let point: NSPoint = unsafe {
        msg_send![view, convertPoint: window_point, fromView: std::ptr::null::<NSView>()]
    };
    (point.x, point.y)
}

struct PdfMotion {
    slide: crate::visit::Slide,
    started: Instant,
    /// Zero is a key, Space, or an edge. A trackpad token is taken when the
    /// slide lands, and a retired token does not write.
    token: u64,
}

struct PdfGlide {
    glide: crate::visit::FingerGlide,
    started: Instant,
}

struct PdfView {
    doc: Document,
    sizes: Vec<(f32, f32)>,
    text: HashMap<usize, PageText>,
    cache: HashMap<(usize, u8), Rgba>,
    frame_ms: Vec<f64>,
}

struct EpubSelection {
    token: u64,
    anchor: Option<Anchor>,
    ready: bool,
    page: u32,
    start: u32,
    end: u32,
    text: String,
    rects: Vec<(f64, f64, f64, f64)>,
    right: f64,
}

struct State {
    context: ComponentContext,
    mode: Mode,
    scale: PixelScale,
    offset: f64,
    pinch: f64,
    drag: Option<((f64, f64), (f64, f64))>,
    pdf: Option<PdfView>,
    book_kind: &'static str,
    book_path: String,
    book_gen: u64,
    drag_gen: u64,
    pdf_motion: Option<PdfMotion>,
    pdf_drag: f64,
    pdf_glide: Option<PdfGlide>,
    pdf_card: (f64, f64),
    pdf_swipe: crate::visit::Swipe,
    product: bool,
    pdf_saved_page: u32,
    pdf_saved_offset: u32,
    page_total: Option<u32>,
    epub_page: usize,
    anchor: Option<Anchor>,
    anchor_page: Option<u32>,
    note: Option<spike_library::margin::Passage>,
    selection_token: u64,
    selection_view: f64,
    selection_view_h: f64,
    selection_gen: u64,
    anchor_payload: String,
    web: Option<crate::web::WebChapter>,
    web_pages: usize,
    trim_id: String,
    declared: Option<(f64, f64)>,
    stage_w: f64,
    stage_h: f64,
    /// The window's content view. The shelf lays out from this, not from the
    /// reader stage, which can stay at the previous frame in full screen.
    shell_w: f64,
    shell_h: f64,
    placed: Option<(i32, i32, i32, i32, String)>,
    placed_zoom: f64,
    placed_stage_w: f64,
    placed_stage_h: f64,
    await_hold: bool,
    settled: bool,
    hold_since: Option<Instant>,
    shown_at: Option<Instant>,
    footer_at: Option<Instant>,
    play_count: u8,
    play_at: Option<Instant>,
    turn_pending: bool,
    seen_frames: usize,
    title_settled: bool,
    chrome: Rc<crate::chrome::Runtime>,
    ease_leg: u64,
    ease_started: Option<Instant>,
    ease_flag: bool,
    ease_sent: bool,
    shelf_leg: u64,
    shelf_started: Option<Instant>,
    shelf_alpha: f64,
    find: Option<FindJob>,
    find_paint: Vec<FindPaint>,
    find_current: Option<(u32, u32, u32)>,
    excerpt_key: String,
    excerpt_text: String,
}

#[derive(Clone)]
struct FindPaint {
    page: usize,
    start: u32,
    end: u32,
    quad: spike_report::Quad,
}

struct PageDecoration {
    anchor_page: Option<u32>,
    notes: Vec<spike_report::Quad>,
    paint: Vec<FindPaint>,
    current: Option<(u32, u32, u32)>,
    scale: PixelScale,
}

/// The characters a search offset counts. The same order the page uses:
/// normalize, then the word-start decoration, then the em-dash joiner.
fn search_chapter(raw: &str) -> String {
    let html = spike_epub::normalize_chapter(raw);
    let bold = crate::settings::bold_word_starts(&crate::settings::path());
    let html = crate::bold_word::decorate(&html, bold);
    spike_epub::dom_text(&html)
}

struct FindJob {
    book: String,
    query: String,
    generation: u64,
    next: usize,
    total: usize,
    ready: bool,
    spine: Vec<(String, String)>,
    hits: Vec<spike_library::column::Hit>,
    text_units: u32,
    empty_units: u32,
    failed_units: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Empty,
    Pdfium,
    Hayro,
    WebKit,
    Blitz,
    Text,
}

impl State {
    fn paint_shell(&self, bounds: NSRect) {
        let Some(ctx) = current_context() else {
            return;
        };
        unsafe {
            let (red, green, blue) = if crate::chrome::shell_on() {
                (235.0 / 255.0, 235.0 / 255.0, 238.0 / 255.0)
            } else {
                crate::reading::paper_rgb()
            };
            CGContext::set_rgb_fill_color(Some(&ctx), red, green, blue, 1.0);
            CGContext::fill_rect(Some(&ctx), bounds);
        }
    }

    fn note_probe(&self, view: &StageView) {
        let window: Option<Retained<NSWindow>> = unsafe { msg_send![view, window] };
        let Some(window) = window else {
            return;
        };
        let (mode, pages, page) = match self.mode {
            Mode::Empty => ("empty", 0, 0),
            Mode::WebKit => ("epub", self.web_pages, self.epub_page + 1),
            Mode::Pdfium | Mode::Hayro => (
                "pdf",
                self.pdf.as_ref().map(|pdf| pdf.sizes.len()).unwrap_or(0),
                self.current_page() + 1,
            ),
            Mode::Text | Mode::Blitz => ("other", 0, 0),
        };
        let close_w = window
            .standardWindowButton(NSWindowButton::CloseButton)
            .map(|button| {
                let view: &NSView = unsafe { &*std::ptr::from_ref(&*button).cast::<NSView>() };
                view.frame().size.width
            })
            .unwrap_or(0.0);
        let toolbar = window.toolbar().is_some_and(|toolbar| {
            toolbar
                .items()
                .iter()
                .any(|item| item.itemIdentifier().to_string() == "books.contents")
        });
        let body = json!({
            "window_title": window.title().to_string(),
            "mode": mode,
            "path": self.book_path,
            "page": page,
            "pages": pages,
            "bar_w": self.chrome.bar_w.get(),
            "lights": self.chrome.lights.get(),
            "close_w": close_w,
            "toolbar": toolbar,
            "product": self.product,
        });
        let dir = "/tmp/superreader-spike";
        let _ = std::fs::create_dir_all(dir);
        let text = serde_json::to_string(&body).unwrap_or_default();
        let path = format!("{dir}/reader.json");
        let tmp = format!("{path}.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }

    fn draw(&mut self, view: &StageView) {
        self.sync_ease(view);
        self.poll_web();
        self.bind_swipe();
        self.note_probe(view);
        let bounds: NSRect = unsafe { msg_send![view, bounds] };
        if bounds.size.width > 1.0 && bounds.size.height > 1.0 {
            self.stage_w = bounds.size.width;
            self.stage_h = bounds.size.height;
        }
        self.paint_shell(bounds);
        if self.mode == Mode::Empty {
            return;
        }
        if self.mode == Mode::WebKit {
            self.stage_w = bounds.size.width;
            self.stage_h = bounds.size.height;
            let pump = self.place_page();
            let playing = std::env::var("SPIKE_PLAY").as_deref() == Ok("1")
                && self.play_count < 3
                && !self.turn_pending;
            if self.chrome_moving() || playing || pump {
                schedule_chrome_pump(view, 0.016);
            }
            return;
        }
        let mtm = MainThreadMarker::new().expect("main");
        let ctx = unsafe {
            let current: Option<Retained<objc2_app_kit::NSGraphicsContext>> =
                msg_send![objc2::class!(NSGraphicsContext), currentContext];
            current.and_then(|g| -> Option<Retained<CGContext>> { msg_send![&*g, CGContext] })
        };
        let Some(ctx) = ctx else {
            return;
        };
        let _ = mtm;
        if !matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            return;
        }
        if self.web.is_some() {
            self.dismiss_web();
        }
        if self.fits_card() {
            self.note_card();
            let pump = self.finish_pdf_motion();
            self.draw_card_pdf(&ctx, bounds);
            if pump {
                schedule_chrome_pump(view, 0.016);
            }
            return;
        }
        let (origin_x, origin_y) = self.pdf_origin();
        let display = self.scale.display() as f64;
        let decoration = self.page_decoration();
        let Some(pdf) = self.pdf.as_mut() else {
            return;
        };
        let mut y = origin_y - self.offset;
        for index in 0..pdf.sizes.len() {
            let (pw, ph) = pdf.sizes[index];
            let dw = pw as f64 * display;
            let dh = ph as f64 * display;
            let next = y + dh;
            if next > 0.0 && y < bounds.size.height {
                let key = (index, scale_key(self.scale));
                if !pdf.cache.contains_key(&key) {
                    let started = std::time::Instant::now();
                    if let Ok(image) = pdf.doc.raster(index, self.scale) {
                        pdf.frame_ms.push(started.elapsed().as_secs_f64() * 1000.0);
                        pdf.cache.insert(key, image);
                    }
                }
                let image = pdf.cache.get(&key).cloned();
                if let Some(image) = image {
                    let mut shown = image;
                    decorate_page(&mut shown, index, &decoration);
                    let bytes = bitmap::flip_rows(shown.width, &shown.bytes);
                    if let Some(cg) = bitmap::rgba_image(
                        shown.width,
                        shown.height,
                        bytes,
                        matches!(self.mode, Mode::Hayro),
                    ) {
                        let rect = NSRect::new(NSPoint::new(origin_x, y), NSSize::new(dw, dh));
                        unsafe { CGContext::draw_image(Some(&ctx), rect, Some(&cg)) };
                    }
                }
            }
            y = next + 16.0;
            if y > bounds.size.height + 40.0 {
                break;
            }
        }
    }

    fn scroll(&mut self, dy: f64) {
        if self.fits_card() && matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            return;
        }
        if matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            spike_library::column::arm_nav(spike_library::column::NavIntent::Trackpad);
            self.offset = (self.offset - dy).max(0.0);
            self.note_page();
            self.publish();
        }
    }

    fn wheel_pdf(&mut self, phase: u64, momentum: u64, dx: f64, dy: f64) {
        self.ensure_forward_size();
        let pages = self.pdf.as_ref().map(|pdf| pdf.sizes.len()).unwrap_or(0) as u32;
        if pages == 0 {
            return;
        }
        let page = self.current_page() as u32 + 1;
        let width = self.fitted_width(self.current_page());
        let effect = crate::visit::offer_while_reading(
            &mut self.pdf_swipe,
            crate::visit::WheelTick {
                finger: crate::visit::finger_bits(phase),
                momentum: momentum != 0,
                dx,
                dy,
            },
            page,
            pages,
            crate::ease::current(),
        );
        let token = self.pdf_swipe.commit_token().unwrap_or(0);
        let committing = self
            .pdf_glide
            .as_ref()
            .is_some_and(|glide| glide.glide.write.is_some());
        let decision = crate::visit::pdf_wheel(
            effect,
            token,
            self.book_gen,
            page,
            pages,
            width,
            self.pdf_drag,
            committing,
            self.pdf_motion.is_some(),
        );
        self.apply_pdf_wheel(decision);
    }

    fn apply_pdf_wheel(&mut self, decision: crate::visit::PdfWheel) {
        if !matches!(decision, crate::visit::PdfWheel::Pass) {
            spike_library::column::arm_nav(spike_library::column::NavIntent::PdfGlide);
        }
        match decision {
            crate::visit::PdfWheel::Pass => {}
            crate::visit::PdfWheel::Follow { shift } => {
                self.pdf_drag = shift;
                if self
                    .pdf_glide
                    .as_ref()
                    .is_some_and(|glide| glide.glide.write.is_none())
                {
                    self.pdf_glide = None;
                }
            }
            crate::visit::PdfWheel::Begin(glide) => {
                self.pdf_drag = 0.0;
                self.pdf_motion = None;
                self.pdf_glide = Some(PdfGlide {
                    glide,
                    started: Instant::now(),
                });
            }
            crate::visit::PdfWheel::Busy { queued } => {
                if queued == 0 {
                    return;
                }
                if let Some(glide) = self.pdf_glide.as_mut() {
                    glide.glide.queued = queued;
                } else if let Some(motion) = self.pdf_motion.as_mut() {
                    crate::visit::queue_slide(&mut motion.slide, queued);
                    if motion.token == 0 {
                        if let Some(token) = self.pdf_swipe.commit_token() {
                            let _ = self.pdf_swipe.take_commit_post(token);
                        }
                    }
                }
            }
            crate::visit::PdfWheel::Drop => self.pdf_drag = 0.0,
        }
    }

    fn note_card(&mut self) {
        let card = self.presented_at(0.0, 0.0).card;
        let (old_w, old_h) = self.pdf_card;
        let finger = self.pdf_drag != 0.0
            || self.pdf_glide.is_some()
            || self
                .pdf_motion
                .as_ref()
                .is_some_and(|motion| motion.token != 0);
        if old_w > 1.0 && finger && !crate::visit::card_keeps_finger(old_w, old_h, card.w, card.h) {
            self.drop_pdf_finger();
        }
        self.pdf_card = (card.w, card.h);
    }

    fn drop_pdf_finger(&mut self) {
        self.pdf_drag = 0.0;
        self.pdf_glide = None;
        if self
            .pdf_motion
            .as_ref()
            .is_some_and(|motion| motion.token != 0)
        {
            self.pdf_motion = None;
        }
        let _ = self.pdf_swipe.cancel_release();
    }

    fn pinch(&mut self, mag: f64) {
        self.pinch += mag;
        if self.pinch > 0.12 {
            self.set_scale(PixelScale::Zoom2x);
            self.pinch = 0.0;
        } else if self.pinch < -0.12 {
            self.set_scale(PixelScale::Retina1x);
            self.pinch = 0.0;
        }
    }

    fn set_scale(&mut self, scale: PixelScale) {
        if self.scale == scale {
            return;
        }
        self.scale = scale;
        if let Some(pdf) = self.pdf.as_mut() {
            pdf.cache.clear();
        }
        self.publish();
    }

    fn finish_drag(&mut self, start: (f64, f64), end: (f64, f64)) {
        if self.drag_gen != self.book_gen {
            return;
        }
        let display = self.scale.display() as f64;
        let (origin_x, origin_y) = self.pdf_origin();
        let Some(pdf) = self.pdf.as_mut() else {
            return;
        };
        let (page, x0, y0) = match locate(
            pdf,
            self.offset,
            start.0,
            start.1,
            display,
            origin_x,
            origin_y,
        ) {
            Some(v) => v,
            None => return,
        };
        let Some((_, x1, y1)) = locate(pdf, self.offset, end.0, end.1, display, origin_x, origin_y)
        else {
            return;
        };
        let text = pdf.text.entry(page).or_insert_with(|| pdf.doc.text(page));
        if let Some(mut anchor) = select_points(text, (x0, y0), (x1, y1)) {
            if let Anchor::Pdf { page: slot, .. } = &mut anchor {
                *slot = page as u32;
            }
            self.store_pdf_anchor(anchor);
            self.publish();
        }
    }

    fn command(&mut self, name: &str, args: &Value) {
        match name {
            "pdf" => {
                let engine = if args.as_str() == Some("hayro") {
                    PdfEngine::Hayro
                } else {
                    PdfEngine::Pdfium
                };
                self.mode = if engine == PdfEngine::Hayro {
                    Mode::Hayro
                } else {
                    Mode::Pdfium
                };
                self.load_pdf(engine);
            }
            "zoom" => {
                let scale = if args.as_str() == Some("zoom-2x") {
                    PixelScale::Zoom2x
                } else {
                    PixelScale::Retina1x
                };
                self.set_scale(scale);
            }
            "book" => {
                self.book_path.clear();
                self.book_kind = if args.as_str() == Some("zhaohua") {
                    "zhaohua"
                } else {
                    "pride"
                };
                self.publish();
            }
            "bold" => {
                let on = args
                    .as_bool()
                    .or_else(|| args.get("on").and_then(|v| v.as_bool()))
                    .unwrap_or(false);
                let _ = crate::settings::store_bold(&crate::settings::path(), on);
                self.find_paint.clear();
                self.find_current = None;
                self.excerpt_text.clear();
                self.excerpt_key.clear();
                if let Some(web) = &self.web {
                    web.clear_find();
                } else {
                    crate::web::set_find_boot("[]");
                }
                let book = args.get("book").and_then(|v| v.as_str()).unwrap_or("");
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let generation = args.get("generation").and_then(|v| v.as_u64()).unwrap_or(0);
                if !book.is_empty() && !query.is_empty() {
                    self.begin_find(book, query, generation);
                } else {
                    self.stop_find();
                }
                if let Some(web) = &self.web {
                    web.word_starts_changed();
                }
            }
            "epub" => {
                self.mode = match args.as_str() {
                    Some("blitz") => Mode::Blitz,
                    Some("text") => Mode::Text,
                    _ => Mode::WebKit,
                };
                self.publish();
            }
            "page" => {
                crate::web::abandon_swipe();
                let delta = args
                    .as_i64()
                    .unwrap_or_else(|| args.get("delta").and_then(|v| v.as_i64()).unwrap_or(0));
                let name = args.get("intent").and_then(|v| v.as_str());
                let dispatched = spike_library::column::page_command(delta, name);
                if let Some(intent) = dispatched.intent {
                    spike_library::column::arm_nav(intent);
                }
                let delta = dispatched.delta;
                if self.mode == Mode::WebKit {
                    if let Some(web) = &self.web {
                        web.turn(delta as i32, crate::visit::TurnSource::User);
                    }
                } else if matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
                    self.turn_pdf(delta);
                } else if delta < 0 {
                    self.epub_page = self.epub_page.saturating_sub(1);
                } else {
                    self.epub_page += 1;
                }
                self.publish();
            }
            "immersive" => self.release_for_ease(),
            "find" => {
                let book = args.get("book").and_then(|v| v.as_str()).unwrap_or("");
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let generation = args.get("generation").and_then(|v| v.as_u64()).unwrap_or(0);
                self.begin_find(book, query, generation);
            }
            "find-stop" => self.stop_find(),
            "jump" => {
                let landing = args
                    .get("landing")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !landing {
                    let intent = match args.get("intent").and_then(|v| v.as_str()) {
                        Some("bookmark") => spike_library::column::NavIntent::Bookmark,
                        _ => spike_library::column::NavIntent::Key,
                    };
                    spike_library::column::arm_nav(intent);
                }
                let href = args.get("href").and_then(|v| v.as_str()).unwrap_or("");
                let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let end = args
                    .get("end")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::from(offset)) as u32;
                self.jump_place(href, page, offset, end);
            }
            _ => {}
        }
    }

    fn begin_find(&mut self, book: &str, query: &str, generation: u64) {
        self.stop_find();
        let query = query.trim();
        if book.is_empty() || query.is_empty() || book != self.book_path {
            return;
        }
        self.find = Some(FindJob {
            book: book.to_string(),
            query: query.to_string(),
            generation,
            next: 0,
            total: 0,
            ready: self.web.is_none(),
            spine: Vec::new(),
            hits: Vec::new(),
            text_units: 0,
            empty_units: 0,
            failed_units: 0,
        });
    }

    fn stop_find(&mut self) {
        self.find = None;
        self.find_paint.clear();
        self.find_current = None;
        if let Some(web) = &self.web {
            web.clear_find();
        }
        crate::web::set_find_boot("[]");
    }

    fn pump_find(&mut self) -> bool {
        if self.find.is_none() {
            return false;
        }
        let book = self
            .find
            .as_ref()
            .map(|job| job.book.clone())
            .unwrap_or_default();
        if book != self.book_path {
            self.stop_find();
            return true;
        }
        let done = self
            .find
            .as_ref()
            .is_some_and(|job| job.total > 0 && job.next >= job.total);
        if done {
            return false;
        }
        if self.web.is_some() {
            self.pump_epub_find()
        } else if self.pdf.is_some() {
            self.pump_pdf_find()
        } else {
            self.fail_find()
        }
    }

    fn fail_find(&mut self) -> bool {
        if let Some(job) = self.find.as_mut() {
            job.total = 1;
            job.next = 1;
            job.failed_units = 1;
            job.ready = true;
        }
        true
    }

    fn pump_pdf_find(&mut self) -> bool {
        let index = self.find.as_ref().map(|job| job.next).unwrap_or(0);
        let total = self
            .pdf
            .as_ref()
            .map(|pdf| pdf.doc.page_count())
            .unwrap_or(0);
        if total == 0 {
            return self.fail_find();
        }
        if let Some(job) = self.find.as_mut() {
            job.total = total;
        }
        if index >= total {
            return false;
        }
        let query = self
            .find
            .as_ref()
            .map(|job| job.query.clone())
            .unwrap_or_default();
        let read = {
            let Some(pdf) = self.pdf.as_ref() else {
                return self.fail_find();
            };
            pdf.doc.search_page(index)
        };
        match read {
            PageRead::Text(text) => {
                let hits = spike_library::column::hits_in(
                    &text.plain,
                    &query,
                    index as u32 + 1,
                    "",
                    spike_library::column::IndexKind::Chars,
                );
                for hit in &hits {
                    for glyph in text
                        .glyphs
                        .iter()
                        .filter(|glyph| glyph.index >= hit.start && glyph.index < hit.end)
                    {
                        self.find_paint.push(FindPaint {
                            page: index,
                            start: hit.start,
                            end: hit.end,
                            quad: glyph.quad,
                        });
                    }
                }
                if let Some(job) = self.find.as_mut() {
                    job.text_units = job.text_units.saturating_add(1);
                    job.hits.extend(hits);
                    job.next += 1;
                }
                if let Some(pdf) = self.pdf.as_mut() {
                    pdf.text.insert(index, text);
                }
            }
            PageRead::Empty => {
                if let Some(job) = self.find.as_mut() {
                    job.empty_units = job.empty_units.saturating_add(1);
                    job.next += 1;
                }
            }
            PageRead::Failed(_) => {
                if let Some(job) = self.find.as_mut() {
                    job.failed_units = job.failed_units.saturating_add(1);
                    job.next += 1;
                }
            }
        }
        true
    }

    fn pump_epub_find(&mut self) -> bool {
        let ready = self.find.as_ref().is_some_and(|job| job.ready);
        if !ready {
            let path = self.book_path.clone();
            match spike_epub::pack(&path) {
                Ok(packed) => {
                    let spine = packed
                        .spine
                        .iter()
                        .filter_map(|href| {
                            let bytes = packed.files.get(href)?;
                            let raw = String::from_utf8_lossy(bytes);
                            let text = search_chapter(&raw);
                            Some((href.clone(), text))
                        })
                        .collect::<Vec<_>>();
                    if let Some(job) = self.find.as_mut() {
                        job.total = spine.len();
                        job.ready = true;
                        job.spine = spine;
                        if job.total == 0 {
                            job.empty_units = 1;
                            job.total = 1;
                            job.next = 1;
                        }
                    }
                }
                Err(_) => return self.fail_find(),
            }
        }
        let index = self.find.as_ref().map(|job| job.next).unwrap_or(0);
        let total = self.find.as_ref().map(|job| job.total).unwrap_or(0);
        if index >= total {
            return true;
        }
        let unit = self
            .find
            .as_ref()
            .and_then(|job| job.spine.get(index).cloned());
        let Some((href, text)) = unit else {
            return true;
        };
        let query = self
            .find
            .as_ref()
            .map(|job| job.query.clone())
            .unwrap_or_default();
        let web_href = self
            .web
            .as_ref()
            .and_then(|web| {
                web.spine().into_iter().find(|item| {
                    spike_epub::normalize_href(item) == spike_epub::normalize_href(&href)
                })
            })
            .unwrap_or(href);
        let unit = self
            .web
            .as_ref()
            .and_then(|web| {
                web.spine().iter().position(|item| {
                    spike_epub::normalize_href(item) == spike_epub::normalize_href(&web_href)
                })
            })
            .map(|at| at as u32 + 1)
            .unwrap_or(index as u32 + 1);
        let hits = if text.trim().is_empty() {
            Vec::new()
        } else {
            spike_library::column::hits_in(
                &text,
                &query,
                unit,
                &web_href,
                spike_library::column::IndexKind::Utf16,
            )
        };
        if let Some(job) = self.find.as_mut() {
            if text.trim().is_empty() {
                job.empty_units = job.empty_units.saturating_add(1);
            } else {
                job.text_units = job.text_units.saturating_add(1);
                job.hits.extend(hits);
            }
            job.next += 1;
        }
        self.paint_epub_hits();
        true
    }

    fn paint_epub_hits(&self) {
        let Some(web) = &self.web else {
            return;
        };
        let shown = spike_epub::normalize_href(&web.href());
        let current = self.find_current;
        let hits = self
            .find
            .as_ref()
            .map(|job| {
                job.hits
                    .iter()
                    .filter(|hit| spike_epub::normalize_href(&hit.href) == shown)
                    .map(|hit| {
                        json!({
                            "start": hit.start,
                            "end": hit.end,
                            "current": current.is_some_and(|(_, start, end)| hit.start == start && hit.end == end),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        web.paint_find(&Value::Array(hits).to_string());
    }

    fn jump_place(&mut self, href: &str, page: u32, offset: u32, end: u32) {
        self.find_current = Some((page, offset, end));
        let here = self.visit_place();
        let dest = crate::visit::ReadingPlace {
            href: href.to_string(),
            page,
            offset,
        };
        let spine = self.web.as_ref().map(|web| web.spine()).unwrap_or_default();
        let step = crate::visit::place_jump(&here, &dest, &spine, crate::web::reader_back());
        crate::web::set_reader_back(step.back);
        match step.go {
            crate::visit::ChapterMove::Reveal { offset } => {
                if let Some(web) = &self.web {
                    web.choose_sentence(offset);
                }
            }
            crate::visit::ChapterMove::Chapter { index, offset } => {
                crate::web::arm_jump(href, offset);
                if let Some(web) = &self.web {
                    let _ = web.show_chapter(index);
                }
            }
            crate::visit::ChapterMove::Page { page } => {
                let _ = self.show_pdf_page(page);
            }
            crate::visit::ChapterMove::Stay if href.is_empty() => {
                let _ = self.show_pdf_page(page);
            }
            crate::visit::ChapterMove::Stay => {
                if let Some(web) = &self.web {
                    web.retire_open_document();
                }
            }
        }
        self.paint_epub_hits();
        self.publish();
    }

    fn reading_excerpt(&mut self) -> String {
        let place = self.visit_place();
        if place.href.is_empty() {
            let index = place.page.saturating_sub(1) as usize;
            return excerpt_window(&self.pdf_plain(index), 0);
        }
        let key = format!("{}|{}", self.book_path, place.href);
        if self.excerpt_key != key {
            let text = self.chapter_text(&place.href);
            self.excerpt_text = text;
            self.excerpt_key = key;
        }
        excerpt_window(&self.excerpt_text, place.offset)
    }

    fn pdf_plain(&mut self, index: usize) -> String {
        if let Some(pdf) = self.pdf.as_ref() {
            if let Some(text) = pdf.text.get(&index) {
                return text.plain.clone();
            }
        }
        let Some(pdf) = self.pdf.as_mut() else {
            return String::new();
        };
        let text = pdf.doc.text(index);
        let plain = text.plain.clone();
        pdf.text.insert(index, text);
        plain
    }

    fn chapter_text(&self, href: &str) -> String {
        let Ok(packed) = spike_epub::pack(&self.book_path) else {
            return String::new();
        };
        let want = spike_epub::normalize_href(href);
        let Some(key) = packed
            .spine
            .iter()
            .find(|item| spike_epub::normalize_href(item) == want)
        else {
            return String::new();
        };
        let Some(bytes) = packed.files.get(key) else {
            return String::new();
        };
        let raw = String::from_utf8_lossy(bytes);
        search_chapter(&raw)
    }

    fn visit_place(&self) -> crate::visit::ReadingPlace {
        if let Some(web) = &self.web {
            let place = crate::web::reader_place();
            if !place.href.is_empty() {
                return place;
            }
            return crate::visit::ReadingPlace {
                href: web.href(),
                page: place.page.max(1),
                offset: place.offset,
            };
        }
        crate::visit::ReadingPlace {
            href: String::new(),
            page: self.current_page() as u32 + 1,
            offset: 0,
        }
    }

    fn show_pdf_page(&mut self, page: u32) -> bool {
        self.ensure_pdf_sizes(page);
        let sizes = {
            let Some(pdf) = self.pdf.as_ref() else {
                return false;
            };
            if page == 0 || page as usize > pdf.sizes.len() {
                return false;
            }
            pdf.sizes.clone()
        };
        self.offset = page_offset(&sizes, page, self.scale);
        let path = crate::settings::path();
        let mut store = crate::settings::SettingsBook {
            file: &path,
            book: &self.book_path,
        };
        let _ = crate::visit::apply_contents_choice(
            &mut self.pdf_glide,
            &mut self.pdf_motion,
            &mut self.pdf_drag,
            &mut self.pdf_saved_page,
            &mut self.pdf_saved_offset,
            &crate::visit::ChapterMove::Page { page },
            &mut store,
        );
        true
    }

    /// Open measures at most 80 pages. A link can land past that, and the
    /// next turn needs the following page's size or it stays put.
    fn ensure_forward_size(&mut self) {
        let Some(pdf) = self.pdf.as_ref() else {
            return;
        };
        let count = pdf.doc.page_count();
        if count == 0 {
            return;
        }
        let index = self.current_page().min(count - 1);
        let next = (index + 1).min(count - 1);
        self.ensure_pdf_sizes(next as u32 + 1);
    }

    fn ensure_pdf_sizes(&mut self, page: u32) {
        let Some(pdf) = self.pdf.as_mut() else {
            return;
        };
        let count = pdf.doc.page_count();
        let need = (page as usize).min(count);
        while pdf.sizes.len() < need {
            let index = pdf.sizes.len();
            pdf.sizes.push(pdf.doc.page_size_pt(index));
        }
    }

    fn find_snapshot(&self) -> Value {
        let Some(job) = &self.find else {
            return Value::Null;
        };
        json!({
            "book": job.book,
            "query": job.query,
            "generation": job.generation,
            "seen": job.next,
            "total": job.total,
            "text_units": job.text_units,
            "empty_units": job.empty_units,
            "failed_units": job.failed_units,
            "done": job.total > 0 && job.next >= job.total,
            "nav": spike_library::column::nav_epoch(),
            "hits": job.hits.iter().map(|hit| json!({
                "page": hit.page,
                "href": hit.href,
                "start": hit.start,
                "end": hit.end,
                "kind": match hit.kind {
                    spike_library::column::IndexKind::Chars => "chars",
                    spike_library::column::IndexKind::Utf16 => "utf16",
                },
                "snippet": hit.snippet,
            })).collect::<Vec<_>>(),
        })
    }

    fn page_decoration(&self) -> PageDecoration {
        let notes = match &self.anchor {
            Some(Anchor::Pdf { quads, .. }) => quads.clone(),
            _ => Vec::new(),
        };
        PageDecoration {
            anchor_page: self.anchor_page,
            notes,
            paint: self.find_paint.clone(),
            current: self.find_current,
            scale: self.scale,
        }
    }

    fn apply_chapter(&mut self, tag: isize) {
        let Some(web) = self.web.as_ref() else {
            if tag < 0 {
                self.return_saved_place();
            }
            return;
        };
        let spine = web.spine();
        let marks = crate::visit::contents_marks(&web.book_title, &spine);
        let href = web.href();
        let decided = crate::web::turn_chapter(tag, &marks, &spine, &href);
        if !decided.accepted {
            return;
        }
        spike_library::column::arm_nav(spike_library::column::NavIntent::Toc);
        crate::web::cancel_swipe();
        let _ = self.pdf_swipe.cancel_release();
        let go = decided
            .go
            .clone()
            .unwrap_or(crate::visit::ChapterMove::Stay);
        let path = crate::settings::path();
        let mut store = crate::settings::SettingsBook {
            file: &path,
            book: &self.book_path,
        };
        let saved = crate::visit::apply_contents_choice(
            &mut self.pdf_glide,
            &mut self.pdf_motion,
            &mut self.pdf_drag,
            &mut self.pdf_saved_page,
            &mut self.pdf_saved_offset,
            &go,
            &mut store,
        );
        let moved = match &go {
            crate::visit::ChapterMove::Stay => {
                if self.mode == Mode::WebKit {
                    web.retire_open_document();
                }
                true
            }
            crate::visit::ChapterMove::Reveal { offset } => {
                let mut inbox = web.inbox.borrow_mut();
                inbox.pinned = None;
                inbox.pin_missed = false;
                drop(inbox);
                web.choose_sentence(*offset);
                true
            }
            crate::visit::ChapterMove::Chapter { index, .. } => web.show_chapter(*index).is_some(),
            crate::visit::ChapterMove::Page { page } => {
                if matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
                    if let Some(pdf) = self.pdf.as_ref() {
                        let sizes = pdf.sizes.clone();
                        self.offset = page_offset(&sizes, *page, self.scale);
                        if !saved {
                            self.note_page();
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        if moved && matches!(go, crate::visit::ChapterMove::Chapter { .. }) {
            self.anchor_payload.clear();
            self.clear_selection();
        }
        let _ = (moved, go);
    }

    fn return_saved_place(&mut self) {
        let Some(back) = crate::web::reader_back() else {
            return;
        };
        if back.href.is_empty() {
            spike_library::column::arm_nav(spike_library::column::NavIntent::Toc);
            if self.show_pdf_page(back.page) {
                crate::web::set_reader_back(None);
                self.publish();
            }
        }
    }

    fn select_phrase(&mut self, phrase: &str) {
        let Some(pdf) = self.pdf.as_mut() else {
            return;
        };
        let text = pdf.text.entry(0).or_insert_with(|| pdf.doc.text(0));
        let Some(byte) = text.plain.find(phrase) else {
            return;
        };
        let start = text.plain[..byte].chars().count() as u32;
        let end = start + phrase.chars().count() as u32;
        let quads: Vec<spike_report::Quad> = text
            .glyphs
            .iter()
            .filter(|glyph| glyph.index >= start && glyph.index < end)
            .map(|glyph| glyph.quad)
            .collect();
        self.store_pdf_anchor(Anchor::Pdf {
            page: 0,
            start,
            end,
            text: phrase.into(),
            quads,
        });
        self.publish();
    }

    fn drag_two_lines(&mut self) {
        let dragged = {
            let Some(pdf) = self.pdf.as_mut() else {
                return;
            };
            let text = pdf.text.entry(0).or_insert_with(|| pdf.doc.text(0));
            let Some(at) = text.plain.find("Recent") else {
                return;
            };
            let start = text.plain[..at].chars().count() as u32;
            let Some(first) = text.glyphs.iter().find(|glyph| glyph.index == start) else {
                return;
            };
            let y0 = first.quad.y;
            let line_y = f64::from(y0);
            let first_pt = (first.quad.x + 1.0, first.quad.y + first.quad.h * 0.5);
            let mut end = start + 20;
            let mut seen_next = false;
            for glyph in text.glyphs.iter().filter(|glyph| glyph.index > start) {
                if !seen_next && glyph.quad.y > y0 + 6.0 {
                    seen_next = true;
                }
                if seen_next {
                    end = glyph.index;
                    if glyph.index > start + 80 && glyph.quad.y > y0 + 6.0 {
                        break;
                    }
                }
            }
            let Some(last) = text.glyphs.iter().find(|glyph| glyph.index == end) else {
                return;
            };
            let last_pt = (last.quad.x + 1.0, last.quad.y + last.quad.h * 0.5);
            let Some(mut anchor) = select_points(text, first_pt, last_pt) else {
                return;
            };
            if let Anchor::Pdf { page, .. } = &mut anchor {
                *page = 0;
            }
            (anchor, line_y)
        };
        let (anchor, line_y) = dragged;
        self.store_pdf_anchor(anchor);
        let display = self.scale.display() as f64;
        self.offset = (line_y * display - 36.0).max(0.0);
        if let Some(Anchor::Pdf { text, .. }) = &self.anchor {
            let _ = std::fs::write("/tmp/superreader-spike/drag.txt", text);
        }
    }

    fn load_pdf(&mut self, engine: PdfEngine) {
        let path = spike_report::gpt3_path();
        let _ = self.load_pdf_at(engine, &path, 1);
        self.publish();
    }

    fn fits_card(&self) -> bool {
        self.product || crate::chrome::shell_on()
    }

    fn fit_box(&self) -> (f64, f64, f64, f64) {
        if self.fits_card() {
            let page = self.presented_at(0.0, 0.0).page;
            (page.w, page.h, page.x, page.y)
        } else {
            crate::chrome::content_box(self.stage_w, self.stage_h)
        }
    }

    fn presented_at(&self, width: f64, height: f64) -> crate::ease::Presented {
        let (width, height) = if width > 1.0 {
            (width, height)
        } else if self.stage_w > 2.0 {
            (self.stage_w, self.stage_h)
        } else {
            (self.shell_w, self.shell_h)
        };
        crate::ease::present(width, height, 0.0, crate::ease::current())
    }

    fn sync_ease(&mut self, view: &StageView) {
        let trip = crate::ease::current();
        if trip.leg() != self.ease_leg {
            self.ease_leg = trip.leg();
            self.ease_started = Some(Instant::now());
        }
        let trip_elapsed = if trip.moving() {
            self.ease_started
                .map(|started| started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32)
                .unwrap_or(0)
        } else {
            0
        };
        let shelf_leg = crate::shelf_pass::leg();
        if shelf_leg != self.shelf_leg {
            self.shelf_leg = shelf_leg;
            self.shelf_started = Some(Instant::now());
        }
        let shelf_elapsed = if crate::shelf_pass::moving() {
            self.shelf_started
                .map(|started| started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32)
                .unwrap_or(0)
        } else {
            0
        };
        let pumped = pump_ease(trip_elapsed, shelf_elapsed);
        if pumped.trip_moving {
            schedule_chrome_pump(view, 0.016);
        }
        if pumped.shelf_moving {
            schedule_chrome_pump(view, 0.016);
        }
        if pumped.emit {
            self.context.emit(
                "ease",
                json!({
                    "u": pumped.trip.u(),
                    "leg": pumped.trip.leg(),
                    "shelf": crate::shelf_pass::progress(),
                    "shelf_leg": crate::shelf_pass::leg(),
                }),
            );
        }
        self.sync_web_opacity();
        self.mark_ease(pumped.trip_moving);
    }

    fn sync_web_opacity(&mut self) {
        let alpha = crate::shelf_pass::body_alpha();
        if (self.shelf_alpha - alpha).abs() < 0.002 {
            return;
        }
        self.shelf_alpha = alpha;
        if let Some(web) = &self.web {
            web.set_opacity(alpha);
        }
    }

    fn release_for_ease(&mut self) {
        self.pdf_drag = 0.0;
        self.pdf_glide = None;
        self.pdf_motion = None;
        let _ = crate::visit::stop_for_ease(&mut self.pdf_swipe);
        crate::web::cancel_swipe();
        self.mark_ease(crate::ease::current().moving());
    }

    fn mark_ease(&mut self, moving: bool) {
        if self.ease_flag == moving && self.ease_sent {
            return;
        }
        self.ease_flag = moving;
        if let Some(web) = &self.web {
            web.set_ease(moving);
            self.ease_sent = true;
        }
    }

    fn draw_card_pdf(&mut self, ctx: &CGContext, bounds: NSRect) {
        let _ = bounds;
        let shown = self.presented_at(0.0, 0.0);
        let card = shown.card;
        let page = shown.page;
        fill_card(
            ctx,
            card.x,
            card.y,
            card.w,
            card.h,
            shown.radius,
            shown.shadow,
        );
        let count = self.pdf.as_ref().map(|pdf| pdf.sizes.len()).unwrap_or(0);
        if count == 0 {
            return;
        }
        let index = self.current_page().min(count - 1);
        let (from, incoming, shift) = self.pdf_shift(index);
        let from = from.min(count - 1);
        let Some((pw, ph)) = self
            .pdf
            .as_ref()
            .and_then(|pdf| pdf.sizes.get(from).copied())
        else {
            return;
        };
        if pw < 1.0 || ph < 1.0 {
            return;
        }
        let (x, y, dw, dh) = spike_report::fitted_pdf_page(
            page.x,
            page.y,
            page.w,
            page.h,
            f64::from(pw),
            f64::from(ph),
        );
        let key_forward = self
            .pdf_motion
            .as_ref()
            .map(|motion| motion.slide.forward)
            .unwrap_or(true);
        let forward = crate::visit::neighbor_on_the_right(shift, key_forward);
        let next_page = incoming.and_then(|next| {
            let (nw, nh) = self.pdf.as_ref()?.sizes.get(next).copied()?;
            if nw < 1.0 || nh < 1.0 {
                return None;
            }
            let fitted = spike_report::fitted_pdf_page(
                page.x,
                page.y,
                page.w,
                page.h,
                f64::from(nw),
                f64::from(nh),
            );
            Some((next, fitted))
        });
        unsafe {
            CGContext::save_g_state(Some(ctx));
            CGContext::clip_to_rect(
                Some(ctx),
                objc2_core_foundation::CGRect::new(
                    objc2_core_foundation::CGPoint::new(card.x, card.y),
                    objc2_core_foundation::CGSize::new(card.w, card.h),
                ),
            );
            CGContext::set_alpha(Some(ctx), crate::shelf_pass::body_alpha());
        }
        self.paint_pdf_page(ctx, from, x - shift, y, dw, dh);
        if let Some((next, (nx, ny, ndw, ndh))) = next_page {
            let step = if forward { dw } else { -dw };
            self.paint_pdf_page(ctx, next, nx - shift + step, ny, ndw, ndh);
        }
        unsafe { CGContext::restore_g_state(Some(ctx)) };
    }

    fn pdf_shift(&self, index: usize) -> (usize, Option<usize>, f64) {
        if let Some(motion) = &self.pdf_motion {
            let elapsed = motion
                .started
                .elapsed()
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            match crate::visit::slide_at(&motion.slide, elapsed, self.book_gen, 1.0) {
                crate::visit::SlideAt::Moving { page, .. } => {
                    let width = self.fitted_width(page);
                    let shift = match crate::visit::slide_at(
                        &motion.slide,
                        elapsed,
                        self.book_gen,
                        width,
                    ) {
                        crate::visit::SlideAt::Moving { shift, .. } => shift,
                        _ => 0.0,
                    };
                    return (page, Some(motion.slide.to), shift);
                }
                _ => return (index, None, 0.0),
            }
        }
        if let Some(glide) = &self.pdf_glide {
            let elapsed = glide
                .started
                .elapsed()
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            return match crate::visit::glide_at(&glide.glide, elapsed, self.book_gen) {
                crate::visit::GlideAt::Moving {
                    page,
                    shift,
                    incoming,
                } => (page, incoming, shift),
                _ => (index, None, 0.0),
            };
        }
        if self.pdf_drag != 0.0 {
            let pages = self.pdf.as_ref().map(|pdf| pdf.sizes.len()).unwrap_or(0) as u32;
            let incoming = crate::visit::finger_neighbor(index as u32 + 1, pages, self.pdf_drag);
            return (index, incoming, self.pdf_drag);
        }
        (index, None, 0.0)
    }

    fn fitted_width(&self, index: usize) -> f64 {
        let Some(pdf) = self.pdf.as_ref() else {
            return 0.0;
        };
        let Some((pw, ph)) = pdf.sizes.get(index).copied() else {
            return 0.0;
        };
        let page = self.presented_at(0.0, 0.0).page;
        let (_, _, dw, _) = spike_report::fitted_pdf_page(
            page.x,
            page.y,
            page.w,
            page.h,
            f64::from(pw),
            f64::from(ph),
        );
        dw
    }

    fn paint_pdf_page(&mut self, ctx: &CGContext, index: usize, x: f64, y: f64, dw: f64, dh: f64) {
        let key = (index, scale_key(self.scale));
        let decoration = self.page_decoration();
        let Some(pdf) = self.pdf.as_mut() else {
            return;
        };
        if !pdf.cache.contains_key(&key) {
            if let Ok(image) = pdf.doc.raster(index, self.scale) {
                pdf.cache.insert(key, image);
            }
        }
        let image = pdf.cache.get(&key).cloned();
        let Some(image) = image else {
            return;
        };
        let mut shown = image;
        decorate_page(&mut shown, index, &decoration);
        let bytes = bitmap::flip_rows(shown.width, &shown.bytes);
        if let Some(cg) = bitmap::rgba_image(
            shown.width,
            shown.height,
            bytes,
            matches!(self.mode, Mode::Hayro),
        ) {
            let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(dw, dh));
            unsafe { CGContext::draw_image(Some(ctx), rect, Some(&cg)) };
        }
    }

    fn finish_pdf_motion(&mut self) -> bool {
        let key = self.finish_key_slide();
        let finger = self.finish_finger_glide();
        key || finger
    }

    fn finish_key_slide(&mut self) -> bool {
        let Some(motion) = self.pdf_motion.take() else {
            return false;
        };
        let elapsed = motion
            .started
            .elapsed()
            .as_millis()
            .min(u128::from(u32::MAX)) as u32;
        match crate::visit::pdf_turn(&motion.slide, elapsed, self.book_gen) {
            crate::visit::PdfTurn::Dropped => false,
            crate::visit::PdfTurn::Moving { .. } => {
                self.pdf_motion = Some(motion);
                true
            }
            crate::visit::PdfTurn::Landed { page, queued } => {
                let accept = crate::visit::accept_landed(&mut self.pdf_swipe, motion.token);
                if accept {
                    if let Some(pdf) = self.pdf.as_ref() {
                        let sizes = pdf.sizes.clone();
                        self.offset = page_offset(&sizes, page as u32 + 1, self.scale);
                    }
                    self.note_page();
                    self.publish();
                }
                if queued != 0 {
                    let token = self.pdf_swipe.commit_token().unwrap_or(0);
                    self.begin_pdf_slide(i64::from(queued), token);
                }
                self.pdf_motion.is_some()
            }
        }
    }

    fn finish_finger_glide(&mut self) -> bool {
        let Some(glide) = self.pdf_glide.take() else {
            return false;
        };
        let elapsed = glide
            .started
            .elapsed()
            .as_millis()
            .min(u128::from(u32::MAX)) as u32;
        match crate::visit::glide_at(&glide.glide, elapsed, self.book_gen) {
            crate::visit::GlideAt::Dropped => false,
            crate::visit::GlideAt::Moving { .. } => {
                self.pdf_glide = Some(glide);
                true
            }
            crate::visit::GlideAt::Landed {
                page,
                write,
                queued,
                token,
            } => {
                let accept =
                    write.is_some() && crate::visit::accept_landed(&mut self.pdf_swipe, token);
                if accept {
                    if let Some(pdf) = self.pdf.as_ref() {
                        let sizes = pdf.sizes.clone();
                        self.offset = page_offset(&sizes, page as u32 + 1, self.scale);
                    }
                    self.note_page();
                    self.publish();
                }
                if queued != 0 {
                    let next = self.pdf_swipe.commit_token().unwrap_or(0);
                    self.begin_pdf_slide(i64::from(queued), next);
                }
                self.pdf_motion.is_some()
            }
        }
    }

    fn start_pdf_slide(&mut self, delta: i64) -> bool {
        self.begin_pdf_slide(delta, 0)
    }

    fn begin_pdf_slide(&mut self, delta: i64, token: u64) -> bool {
        self.ensure_forward_size();
        let Some(pdf) = self.pdf.as_ref() else {
            return false;
        };
        let count = pdf.sizes.len();
        let Some(slide) =
            crate::visit::begin_slide(self.book_gen, count, self.current_page(), delta as i32)
        else {
            return false;
        };
        self.pdf_drag = 0.0;
        self.pdf_glide = None;
        self.pdf_motion = Some(PdfMotion {
            slide,
            started: Instant::now(),
            token,
        });
        if token != 0 {
            self.pdf_swipe.note_answer(token, "glide");
        }
        true
    }

    fn load_pdf_at(&mut self, engine: PdfEngine, path: &str, page: u32) -> bool {
        let bytes = std::fs::read(path).unwrap_or_default();
        if bytes.is_empty() {
            self.pdf = None;
            return false;
        }
        let lib = spike_report::pdfium_lib();
        let opened = Document::open(engine, bytes.clone(), &lib)
            .map(|(doc, _, _)| (doc, engine))
            .or_else(|_| {
                let other = if engine == PdfEngine::Hayro {
                    PdfEngine::Pdfium
                } else {
                    PdfEngine::Hayro
                };
                Document::open(other, bytes, &lib).map(|(doc, _, _)| (doc, other))
            });
        let Ok((doc, used)) = opened else {
            self.pdf = None;
            return false;
        };
        self.page_total = Some(doc.page_count() as u32);
        let count = doc.page_count().min(80);
        let sizes: Vec<_> = (0..count).map(|i| doc.page_size_pt(i)).collect();
        if sizes.is_empty() || sizes.iter().all(|(w, h)| *w < 1.0 || *h < 1.0) {
            self.pdf = None;
            return false;
        }
        self.mode = match used {
            PdfEngine::Hayro => Mode::Hayro,
            PdfEngine::Pdfium => Mode::Pdfium,
        };
        self.offset = page_offset(&sizes, page, self.scale);
        self.clear_selection();
        self.anchor_payload.clear();
        self.pdf = Some(PdfView {
            doc,
            sizes,
            text: HashMap::new(),
            cache: HashMap::new(),
            frame_ms: Vec::new(),
        });
        true
    }

    fn turn_pdf(&mut self, delta: i64) {
        let _ = self.pdf_swipe.abandon();
        self.pdf_drag = 0.0;
        if let Some(glide) = self.pdf_glide.as_mut() {
            if glide.glide.write.is_some() && glide.glide.book == self.book_gen {
                glide.glide.queued = if delta < 0 { -1 } else { 1 };
                if let Some(token) = self.pdf_swipe.commit_token() {
                    if token != glide.glide.token {
                        self.pdf_swipe.retire(token);
                    }
                }
                return;
            }
        }
        self.pdf_glide = None;
        if let Some(motion) = self.pdf_motion.as_mut() {
            if motion.slide.book == self.book_gen {
                crate::visit::queue_slide(&mut motion.slide, delta as i32);
                return;
            }
        }
        self.pdf_motion = None;
        self.start_pdf_slide(delta);
    }

    fn open_at(&mut self, view: &StageView, path: &str, page: u32) -> bool {
        let Some(book) = crate::books::parse_book(path) else {
            return false;
        };
        if !crate::books::can_open(&book.path) {
            return false;
        }
        let kind = match book.kind {
            crate::books::BookKind::Pdf => spike_library::OpenKind::Pdf,
            crate::books::BookKind::Epub if std::path::Path::new(&book.path).is_dir() => {
                spike_library::OpenKind::EpubDir
            }
            crate::books::BookKind::Epub => spike_library::OpenKind::EpubFile,
        };
        let mut page = page.max(1);
        self.book_gen = self.book_gen.wrapping_add(1);
        crate::web::set_reader_back(None);
        self.pdf_motion = None;
        self.pdf_drag = 0.0;
        self.pdf_glide = None;
        self.pdf_card = (0.0, 0.0);
        let _ = crate::visit::stop_for_ease(&mut self.pdf_swipe);
        self.page_total = None;
        crate::web::set_book(&book.path);
        let plan = spike_library::OpenSurfaces::closed().open(kind, page);
        self.dismiss_web();
        self.pdf = None;
        self.clear_selection();
        self.anchor_payload.clear();
        self.book_path = book.path.clone();
        self.stop_find();
        let mut replaced_href = String::new();
        let mut offset = 0u32;
        let opened = if plan.pdf {
            let engine = match std::env::var("SPIKE_ENGINE").as_deref() {
                Ok("hayro") => PdfEngine::Hayro,
                _ => PdfEngine::Pdfium,
            };
            self.mode = Mode::Pdfium;
            self.load_pdf_at(engine, &book.path, plan.page)
        } else if plan.web {
            self.mode = Mode::WebKit;
            let sentence = crate::settings::resume_offset(&crate::settings::path(), &book.path);
            let mut href = crate::settings::resume_href(&crate::settings::path(), &book.path);
            let mut caret = crate::settings::resume_caret(&crate::settings::path(), &book.path);
            offset = sentence;
            self.epub_page = 0;
            self.web_pages = 1;
            self.shown_at = None;
            self.placed = None;
            self.open_web(view, &href);
            if let Some(web) = &self.web {
                let open = crate::visit::opened_place(&web.spine(), &href, page, sentence);
                if open.page != page || open.offset != sentence {
                    page = open.page;
                    offset = open.offset;
                    if offset == 0 {
                        caret = false;
                    }
                    let shown = web.href();
                    if !shown.is_empty() {
                        href = shown.clone();
                        replaced_href = shown;
                    }
                }
            }
            if self.web.is_some() {
                crate::web::open_reader(page, offset, &href, caret);
            }
            self.web.is_some()
        } else {
            false
        };
        if !opened {
            crate::web::set_book("");
            self.mode = Mode::Empty;
            self.book_path.clear();
            self.publish();
            return false;
        }
        if matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            self.pdf_saved_page = page;
            self.pdf_saved_offset = 0;
        }
        let _ = crate::settings::record_open(
            &crate::settings::path(),
            &book.path,
            page,
            self.page_total,
            opened_at_ms(),
        );
        if !replaced_href.is_empty() {
            let _ = crate::settings::record_progress(
                &crate::settings::path(),
                &book.path,
                page,
                self.page_total,
                offset,
                &replaced_href,
            );
        }
        self.chrome.reading.set(crate::reading::reader_shown(
            true,
            crate::visit::on_library(),
        ));
        self.chrome.cut_chrome();
        self.publish();
        true
    }

    fn open_requested(&mut self, view: &StageView, path: &str) {
        let page = crate::settings::resume_page(&crate::settings::path(), path);
        self.open_at(view, path, page);
    }

    fn dismiss_web(&mut self) {
        self.ease_sent = false;
        crate::web::cancel_swipe();
        crate::web::show_swipe_target(None);
        if let Some(web) = self.web.take() {
            web.dismiss();
        }
    }

    fn bind_swipe(&self) {
        if self.mode == Mode::WebKit {
            crate::web::show_swipe_target(self.web.clone());
        } else {
            crate::web::show_swipe_target(None);
        }
    }

    fn note_page(&mut self) {
        let Some(shown) = self.settled_page() else {
            return;
        };
        if self.book_path.is_empty() {
            return;
        }
        let Some((page, offset)) =
            crate::visit::landed_progress(self.pdf_saved_page, self.pdf_saved_offset, shown)
        else {
            return;
        };
        let path = self.book_path.clone();
        self.pdf_saved_page = page;
        self.pdf_saved_offset = offset;
        let _ = crate::settings::record_progress(
            &crate::settings::path(),
            &path,
            page,
            self.page_total,
            offset,
            "",
        );
    }

    fn settled_page(&self) -> Option<u32> {
        match self.mode {
            Mode::Pdfium | Mode::Hayro => Some(self.current_page() as u32 + 1),
            Mode::WebKit => {
                let shown = self.epub_page as u32 + 1;
                if crate::web::reader_walk().is_some_and(|target| shown < target) {
                    None
                } else {
                    Some(shown)
                }
            }
            Mode::Empty | Mode::Text | Mode::Blitz => None,
        }
    }

    fn epub_path(&self) -> String {
        if !self.book_path.is_empty() {
            return self.book_path.clone();
        }
        match self.book_kind {
            "zhaohua" => spike_report::zhaohua_path(),
            "styled" | "publisher" | "plain-en" | "plain-zh" | "fixed" => {
                format!("{}/{}.epub", spike_report::fixtures_dir(), self.book_kind)
            }
            _ => spike_report::pride_path(),
        }
    }

    fn poll_web(&mut self) {
        let Some(web) = self.web.clone() else {
            return;
        };
        let (reload, changed, selection) = {
            let inbox = web.inbox.borrow();
            let mut changed = false;
            if let Some((page, pages)) = inbox.pages {
                let page = page.saturating_sub(1);
                if self.epub_page != page || self.web_pages != pages.max(1) {
                    changed = true;
                    self.footer_at = Some(Instant::now());
                }
                self.epub_page = page;
                self.web_pages = pages.max(1);
                if self.shown_at.is_none() && pages > 1 {
                    self.shown_at = Some(Instant::now());
                    self.footer_at = Some(Instant::now());
                }
            }
            let selection = if inbox.selection_token != self.selection_token {
                changed = true;
                Some(EpubSelection {
                    token: inbox.selection_token,
                    anchor: inbox.anchor.clone(),
                    ready: inbox.selection_ready,
                    page: inbox.selection_page,
                    start: inbox.selection_start,
                    end: inbox.selection_end,
                    text: inbox.selection_text.clone(),
                    rects: inbox.selection_rects.clone(),
                    right: inbox.selection_right,
                })
            } else {
                None
            };
            (inbox.reload && !inbox.did_reload, changed, selection)
        };
        if let Some(selection) = selection {
            self.apply_epub(selection);
        }
        if !self.book_path.is_empty() && self.mode == Mode::WebKit {
            if let Some(offset) = crate::web::take_reader_reveal() {
                web.reveal(offset);
            }
        }
        if changed || self.chrome_moving() {
            self.publish();
        }
        if crate::reading::title_alpha(elapsed_ms(self.shown_at)) == 0.0 && self.shown_at.is_some()
        {
            self.title_settled = true;
        }
        if reload {
            web.inbox.borrow_mut().did_reload = true;
            web.inbox.borrow_mut().reload = false;
            web.reload();
        }
        if std::env::var("SPIKE_TURN").as_deref() == Ok("1") && self.frame_settled() {
            let ready = web
                .inbox
                .borrow()
                .pages
                .map(|(_, pages)| pages > 1)
                .unwrap_or(false);
            let already = web.inbox.borrow().scripted_turn;
            if ready && !already {
                web.inbox.borrow_mut().scripted_turn = true;
                self.turn_pending = true;
                web.turn(1, crate::visit::TurnSource::Restore);
            }
        }
        if crate::web::take_turn_declined(&web.inbox) {
            self.turn_pending = false;
        }
        let (page, count) = web.inbox.borrow().pages.unwrap_or((1, 1));
        if crate::web::poll_reader_walk(page as u32, count as u32, self.turn_pending) {
            self.turn_pending = true;
            self.play_at = Some(Instant::now());
            web.turn(1, crate::visit::TurnSource::Restore);
        }
        if self.turn_pending
            && self
                .play_at
                .map(|at| at.elapsed().as_millis() > 700)
                .unwrap_or(false)
        {
            self.turn_pending = false;
        }
        if std::env::var("SPIKE_PLAY").as_deref() == Ok("1") {
            let ready = web
                .inbox
                .borrow()
                .pages
                .map(|(_, pages)| pages > 1)
                .unwrap_or(false);
            let shown = self
                .shown_at
                .map(|at| at.elapsed().as_millis())
                .unwrap_or(0);
            let due = self
                .play_at
                .map(|at| at.elapsed().as_millis() >= 900)
                .unwrap_or(shown >= 1200);
            if ready && self.frame_settled() && self.play_count < 3 && due && !self.turn_pending {
                self.play_count += 1;
                self.play_at = Some(Instant::now());
                self.turn_pending = true;
                web.turn(1, crate::visit::TurnSource::Restore);
            }
        }
        let frame_count = web.inbox.borrow().frame_ms.len();
        if frame_count > self.seen_frames {
            self.seen_frames = frame_count;
            self.turn_pending = false;
        }
        let inbox = web.inbox.borrow();
        let href = web.href();
        crate::web::write_inbox(&inbox, &href);
    }

    fn open_web(&mut self, parent: &StageView, href: &str) {
        let path = self.epub_path();
        let parent: &AnyObject = unsafe { &*(parent as *const StageView).cast() };
        match crate::web::WebChapter::open(parent, &path, href) {
            Ok(chapter) => {
                self.declared = chapter.declared;
                self.placed = None;
                self.web = Some(chapter);
            }
            Err(err) => eprintln!("webkit {err}"),
        }
    }

    fn chrome_moving(&self) -> bool {
        // The window's content height is still changing after the title fades.
        // A native page stops drawing then and keeps the opening frame; the
        // line breaker happens to invalidate again and picks up the final
        // frame, so the two screenshots were different pages.
        self.mode == Mode::WebKit
            && self
                .shown_at
                .map(|at| at.elapsed().as_millis() < 2200)
                .unwrap_or(true)
    }

    fn frame_settled(&self) -> bool {
        self.shown_at
            .map(|at| at.elapsed().as_millis() >= 2200)
            .unwrap_or(false)
            && !self.await_hold
    }

    fn apply_trim(&mut self, id: &str) {
        let id = crate::reading::trim(id).id.to_string();
        if self.trim_id == id {
            return;
        }
        let _ = crate::settings::store(&crate::settings::path(), &id);
        if let Some(web) = &self.web {
            web.trim_changed(&id);
        }
        self.trim_id = id;
        self.placed = None;
        self.place_page();
        self.publish();
    }

    fn reload_trim_from_file(&mut self) -> bool {
        let id = crate::settings::load(&crate::settings::path());
        if id == self.trim_id {
            return false;
        }
        self.apply_trim(&id);
        true
    }

    fn page_box(&self) -> crate::chrome::PageBox {
        let item = crate::reading::trim(&self.trim_id);
        let (rw, rh, _) = crate::reading::page_ratio(self.declared, item);
        let placed = self
            .placed
            .clone()
            .map(|item| (item.0, item.1, item.2, item.3))
            .unwrap_or((0, 0, 0, 0));
        crate::chrome::PageBox {
            x: f64::from(placed.0),
            y: f64::from(placed.1),
            w: f64::from(placed.2),
            h: f64::from(placed.3),
            stage_w: self.stage_w,
            stage_h: self.stage_h,
            ratio_w: rw,
            ratio_h: rh,
        }
    }

    fn place_page(&mut self) -> bool {
        if self.web.is_none() || ((self.stage_w < 1.0 || self.stage_h < 1.0) && !self.await_hold) {
            return false;
        }
        let item = crate::reading::trim(&self.trim_id);
        let (rw, rh, source) = crate::reading::page_ratio(self.declared, item);
        let (box_w, box_h, left, top) = self.fit_box();
        let fitted = crate::chrome::fitted_page(left, top, box_w, box_h, rw, rh);
        let (x, y, w, h) = (fitted.x, fitted.y, fitted.w, fitted.h);
        let design = crate::chrome::reading_page(
            rw,
            rh,
            self.chrome.full.get(),
            &self.chrome.design_visible,
        );
        let zoom = if self.chrome.full.get() && design.1 > 1.0 {
            (h / design.1).max(1.0)
        } else {
            1.0
        };
        let (kx, ky, kw, kh) = fitted.key();
        let key = (kx, ky, kw, kh, self.trim_id.clone());
        let (hold, hold_probe, missed, shown, pages, pages_known, figure, figure_lost) = {
            let inbox = self.web.as_ref().unwrap().inbox.borrow();
            let figure = inbox.figure.clone();
            let figure_lost = inbox.figure_lost;
            match inbox.pages {
                Some((page, count)) => (
                    inbox.hold,
                    inbox.hold_probe,
                    inbox.hold_missed,
                    page as u32,
                    count as u32,
                    true,
                    figure,
                    figure_lost,
                ),
                None => (
                    inbox.hold,
                    inbox.hold_probe,
                    inbox.hold_missed,
                    0,
                    0,
                    false,
                    figure,
                    figure_lost,
                ),
            }
        };
        let (saw_figure, lost_figure) = crate::visit::figure_flags(&figure, figure_lost);
        if let Some(id) = crate::visit::found_figure(figure, figure_lost) {
            crate::web::note_figure(Some(id));
        }
        let timed_out = self.await_hold
            && self
                .hold_since
                .map(|at| at.elapsed().as_millis() > 700)
                .unwrap_or(false);
        let mut tick = crate::visit::FrameTick::new(
            self.placed.as_ref() == Some(&key),
            (self.placed_zoom - zoom).abs() < 0.01,
            self.placed_stage_w.round() as i32 == self.stage_w.round() as i32
                && self.placed_stage_h.round() as i32 == self.stage_h.round() as i32,
            self.declared.is_none(),
            self.settled,
            self.await_hold,
        );
        tick.hold = hold;
        tick.hold_probe = hold_probe;
        tick.missed = missed;
        tick.timed_out = timed_out;
        tick.shown = shown;
        tick.pages = pages;
        tick.pages_known = pages_known;
        tick.figure = saw_figure;
        tick.figure_lost = lost_figure;
        let plan = crate::web::layout_frame(tick);
        let resolved = self.await_hold && !matches!(plan.draw, crate::visit::Draw::Rest);
        let spent = crate::visit::spent_figure_loss(lost_figure, plan.draw)
            || crate::visit::spent_figure_find(saw_figure, plan.draw);
        if resolved || spent {
            if let Some(web) = &self.web {
                let mut inbox = web.inbox.borrow_mut();
                if resolved {
                    inbox.hold = None;
                    inbox.hold_probe = 0;
                    inbox.hold_missed = false;
                }
                inbox.clear_figure();
            }
            if resolved {
                self.await_hold = false;
                self.hold_since = None;
            }
        }
        if plan.cancel {
            crate::web::cancel_placed_page();
            self.placed_stage_w = self.stage_w;
            self.placed_stage_h = self.stage_h;
        }
        match plan.draw {
            crate::visit::Draw::Rest => {}
            crate::visit::Draw::Align => {
                self.settled = true;
                self.placed = Some(key);
                self.placed_zoom = zoom;
                self.placed_stage_w = self.stage_w;
                self.placed_stage_h = self.stage_h;
                if let Some(web) = &self.web {
                    hold_replaced_frame(web, w, h, zoom);
                    web.place_at(x, y, w, h, zoom);
                    web.set_page_target(layout_target(w, zoom));
                    if let Some(id) = crate::web::reader_figure() {
                        web.align(&id);
                    }
                }
            }
            crate::visit::Draw::Measure { page, probe } => {
                self.await_hold = true;
                self.hold_since = Some(Instant::now());
                if let Some(web) = &self.web {
                    {
                        let mut inbox = web.inbox.borrow_mut();
                        inbox.hold = None;
                        inbox.hold_probe = 0;
                        inbox.hold_missed = false;
                        inbox.clear_figure();
                    }
                    web.measure(page, probe);
                }
            }
            crate::visit::Draw::Move { reveal } => {
                self.settled = true;
                self.placed = Some(key);
                self.placed_zoom = zoom;
                self.placed_stage_w = self.stage_w;
                self.placed_stage_h = self.stage_h;
                let Some(web) = &self.web else {
                    return plan.pump;
                };
                hold_replaced_frame(web, w, h, zoom);
                {
                    let mut inbox = web.inbox.borrow_mut();
                    inbox.page_x = x;
                    inbox.bar_w = self.chrome.bar_w.get();
                    inbox.stage_w = self.stage_w;
                    inbox.trim = self.trim_id.clone();
                    inbox.page_source = source.to_string();
                }
                web.place_at(x, y, w, h, zoom);
                web.set_page_target(layout_target(w, zoom));
                let href = web.href();
                crate::web::write_inbox(&web.inbox.borrow(), &href);
                if let Some(offset) = reveal {
                    web.reveal(offset);
                    if let (Some(start), Some(end)) = (
                        self.anchor.as_ref().and_then(anchor_start),
                        self.anchor.as_ref().and_then(anchor_end),
                    ) {
                        web.repaint(start, end);
                    }
                }
            }
        }
        plan.pump
    }

    fn publish(&mut self) {
        self.expire_selection();
        self.sync_anchors();
        let mut title_opacity = 1.0_f32;
        let mut footer_opacity = 1.0_f32;
        let mut bars_row = false;
        let mut shown_page = 0_u32;
        let mut shown_pages = 0_u32;
        let (title, variant, left, right, show_text, html) = match self.mode {
            Mode::Empty => {
                bars_row = true;
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    false,
                    String::new(),
                )
            }
            Mode::Pdfium | Mode::Hayro => {
                let pages = self.pdf.as_ref().map(|p| p.sizes.len()).unwrap_or(0);
                let current = self.current_page().saturating_add(1);
                shown_page = current as u32;
                shown_pages = pages as u32;
                let title = if self.book_path.is_empty() || self.book_path.ends_with("gpt3.pdf") {
                    "Language Models are Few-Shot Learners".to_string()
                } else {
                    std::path::Path::new(&self.book_path)
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .unwrap_or("PDF")
                        .to_string()
                };
                // The installed app uses the EPUB toolbar. The dev spike keeps
                // the engine label and the page row.
                if self.product {
                    bars_row = true;
                }
                let variant = if self.product {
                    String::new()
                } else {
                    format!(
                        "{}  {}",
                        match self.mode {
                            Mode::Hayro => "hayro",
                            _ => "pdfium",
                        },
                        self.scale.name()
                    )
                };
                let (left, right) =
                    crate::reading::shown_page_footer(self.product, current as u32, pages as u32);
                (title, variant, left, right, false, String::new())
            }
            Mode::WebKit => {
                let pages = self.web_pages.max(1);
                let page = (self.epub_page % pages) + 1;
                shown_page = page as u32;
                shown_pages = pages as u32;
                let path = self.epub_path();
                let title = self
                    .web
                    .as_ref()
                    .map(|web| web.book_title.clone())
                    .filter(|title| !title.is_empty())
                    .or_else(|| spike_epub::open_book(&path).ok().map(|book| book.title))
                    .unwrap_or_default();
                title_opacity = crate::reading::title_alpha(elapsed_ms(self.shown_at));
                footer_opacity = 1.0;
                bars_row = true;
                let (left, right) = crate::reading::footer_zh(page as u32, pages as u32);
                (title, String::new(), left, right, false, String::new())
            }
            Mode::Text | Mode::Blitz => {
                let label = match self.mode {
                    Mode::Text => "text view",
                    Mode::WebKit => "webkit",
                    _ => "blitz",
                };
                let path = self.epub_path();
                let book = spike_epub::open_book(&path).ok();
                let title = book.as_ref().map(|b| b.title.clone()).unwrap_or_default();
                let chapter = book.as_ref().and_then(|b| b.chapters.first());
                let html = chapter
                    .map(|c| format!("<p>{}</p>", c.plain))
                    .unwrap_or_default();
                let pages = chapter
                    .map(|c| (c.plain.chars().count() / 1200).max(1))
                    .unwrap_or(1);
                let page = (self.epub_page % pages) + 1;
                shown_page = page as u32;
                shown_pages = pages as u32;
                (
                    title,
                    label.to_string(),
                    format!("{page}/{pages}"),
                    pages_left(pages.saturating_sub(page)),
                    self.mode == Mode::Text,
                    html,
                )
            }
        };
        let entered = crate::reading::enter_reading_chrome(
            crate::chrome::shell_on(),
            self.chrome.reading.get(),
            self.chrome.lights.get() as f32,
            title_opacity,
        );
        title_opacity = entered.title_opacity;
        self.chrome
            .note_chrome_status(f64::from(title_opacity), entered.chrome_shared);
        crate::chrome::append_chrome_status(&self.chrome);
        let (margin_book, margin_spine, margin_page, margin_moving, margin_pw, margin_ph) =
            self.margin_frame();
        let anchor = self
            .anchor
            .as_ref()
            .map(|a| serde_json::to_value(a).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        self.chrome.set_title(&title);
        if self.mode == Mode::WebKit {
            if let Some(web) = &self.web {
                web.set_footer(&left, &right);
            }
        }
        let margin_order = self.web.as_ref().map(|web| web.spine()).unwrap_or_default();
        let margin_excerpt = self.reading_excerpt();
        let mut status = json!({
            "title": title,
            "variant": variant,
            "left": left,
            "right": right,
            "show_text": show_text,
            "html": html,
            "anchor": anchor,
            "title_opacity": title_opacity,
            "chrome_shared": entered.chrome_shared,
            "footer_opacity": footer_opacity,
            "bars_row": bars_row,
            "trim": self.trim_id,
            "page": shown_page,
            "pages": shown_pages,
            "margin_book": margin_book,
            "margin_spine": margin_spine,
            "margin_order": margin_order,
            "margin_excerpt": margin_excerpt,
            "margin_page": margin_page,
            "margin_moving": margin_moving,
            "margin_pw": margin_pw,
            "margin_ph": margin_ph,
            "margin_measures": self.margin_measures(),
            "empty": self.mode == Mode::Empty,
            "stage_w": if self.shell_w > 2.0 { self.shell_w } else { self.stage_w },
            "stage_h": if self.shell_h > 2.0 { self.shell_h } else { self.stage_h },
            "bar_w": self.chrome.bar_w.get(),
            "lights_visible": self.chrome.lights_visible.get(),
            "content_top": self.chrome.content_top.get(),
        });
        status["passage"] = crate::visit::passage_value(self.published_passage());
        status["book_gen"] = json!(self.book_gen);
        status["find"] = self.find_snapshot();
        self.context.emit("status", status);
    }

    fn margin_measures(&self) -> Value {
        let Some(web) = &self.web else {
            return Value::Array(Vec::new());
        };
        let inbox = web.inbox.borrow();
        if !spike_library::margin::measures_ready(inbox.layout_hold, inbox.layout_gen) {
            return Value::Array(Vec::new());
        }
        Value::Array(
            inbox
                .measures
                .iter()
                .map(|measure| {
                    json!({
                        "id": measure.id,
                        "page": measure.page,
                        "text_right": measure.text_right,
                        "rects": measure.quads.iter().map(|quad| json!({
                            "x": quad.x,
                            "y": quad.y,
                            "w": quad.w,
                            "h": quad.h,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        )
    }

    fn expire_selection(&mut self) {
        if self.note.is_none() {
            return;
        }
        let (book, spine, page, moving, _, _) = self.margin_frame();
        let (view, view_h, gen, hold) = self
            .web
            .as_ref()
            .map(|web| {
                let inbox = web.inbox.borrow();
                (
                    inbox.view_w,
                    inbox.view_h,
                    inbox.layout_gen,
                    inbox.layout_hold,
                )
            })
            .unwrap_or((0.0, 0.0, 0, 0));
        let reflow = self.mode == Mode::WebKit
            && spike_library::margin::selection_reflowed(
                self.selection_view,
                self.selection_view_h,
                self.selection_gen,
                view,
                view_h,
                gen,
                hold,
            );
        let stale = self.note.as_ref().is_some_and(|note| {
            moving
                || page == 0
                || note.book != book
                || note.spine != spine
                || note.page != page
                || reflow
        });
        if stale {
            self.clear_selection();
        }
    }

    fn sync_anchors(&mut self) {
        if self.mode != Mode::WebKit {
            return;
        }
        let Some(web) = self.web.clone() else {
            return;
        };
        let href = web.href();
        if self.book_path.is_empty() || href.is_empty() || web.inbox.borrow().await_nav {
            self.anchor_payload.clear();
            return;
        }
        let notes = crate::form::load_spine(&self.book_path, &href).unwrap_or_default();
        let payload = serde_json::to_string(
            &notes
                .iter()
                .map(|note| json!({ "id": note.id, "start": note.start, "end": note.end }))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        if payload == self.anchor_payload {
            return;
        }
        self.anchor_payload = payload.clone();
        web.set_anchors(&payload);
    }

    fn published_passage(&self) -> Option<&spike_library::margin::Passage> {
        self.note
            .as_ref()
            .filter(|note| note.book == self.book_path && note.gen == self.book_gen)
    }

    fn clear_selection(&mut self) {
        self.anchor = None;
        self.anchor_page = None;
        self.note = None;
        self.selection_view = 0.0;
        self.selection_view_h = 0.0;
        self.selection_gen = 0;
    }

    fn store_pdf_anchor(&mut self, anchor: Anchor) {
        let built = match &anchor {
            Anchor::Pdf {
                page,
                start,
                end,
                text,
                quads,
            } => {
                let glyphs = self
                    .pdf
                    .as_ref()
                    .and_then(|pdf| pdf.text.get(&(*page as usize)))
                    .map(|page_text| {
                        page_text
                            .glyphs
                            .iter()
                            .map(|glyph| spike_library::margin::GlyphBox {
                                index: glyph.index,
                                x: f64::from(glyph.quad.x),
                                y: f64::from(glyph.quad.y),
                                w: f64::from(glyph.quad.w),
                                h: f64::from(glyph.quad.h),
                            })
                            .collect::<Vec<_>>()
                    });
                let text_right = glyphs
                    .as_deref()
                    .and_then(|glyphs| spike_library::margin::pdf_line_end(glyphs, *start, *end));
                let quads = quads
                    .iter()
                    .map(|quad| spike_library::margin::Quad {
                        x: f64::from(quad.x),
                        y: f64::from(quad.y),
                        w: f64::from(quad.w),
                        h: f64::from(quad.h),
                    })
                    .collect();
                text_right.and_then(|text_right| {
                    spike_library::margin::passage_from(spike_library::margin::Selection {
                        book: self.book_path.clone(),
                        gen: self.book_gen,
                        page: crate::visit::margin_page_number(*page as usize),
                        spine: String::new(),
                        start: *start,
                        end: *end,
                        text: text.clone(),
                        quads,
                        text_right,
                    })
                })
            }
            Anchor::Epub { .. } => None,
        };
        if let Anchor::Pdf { page, .. } = &anchor {
            let (_, _, shown, moving, _, _) = self.margin_frame();
            if moving || crate::visit::margin_page_number(*page as usize) != shown {
                self.clear_selection();
                return;
            }
            self.anchor_page = Some(*page);
        }
        self.anchor = Some(anchor);
        self.note = built;
    }

    fn apply_epub(&mut self, selection: EpubSelection) {
        self.selection_token = selection.token;
        let (book, spine, page, moving, _, _) = self.margin_frame();
        let href = self.web.as_ref().map(|web| web.href()).unwrap_or_default();
        let view = self
            .web
            .as_ref()
            .map(|web| web.inbox.borrow().view_w)
            .unwrap_or(0.0);
        if !selection.ready
            || moving
            || page == 0
            || selection.page != page
            || href.is_empty()
            || spine != href
            || book != self.book_path
        {
            self.clear_selection();
            return;
        }
        let selected = match &selection.anchor {
            Some(Anchor::Epub { spine_href, .. }) => spine_href.clone(),
            _ => String::new(),
        };
        if selected != href {
            self.clear_selection();
            return;
        }
        self.anchor = selection.anchor.clone();
        self.anchor_page = None;
        self.selection_view = view;
        self.selection_view_h = self
            .web
            .as_ref()
            .map(|web| web.inbox.borrow().view_h)
            .unwrap_or(0.0);
        self.selection_gen = self
            .web
            .as_ref()
            .map(|web| web.inbox.borrow().layout_gen)
            .unwrap_or(0);
        let rects = selection
            .rects
            .iter()
            .copied()
            .map(|(x, y, w, h)| spike_library::margin::MeasuredRect { x, y, w, h })
            .collect::<Vec<_>>();
        self.note = spike_library::margin::passage_from(spike_library::margin::Selection {
            book: self.book_path.clone(),
            gen: self.book_gen,
            page: selection.page,
            spine,
            start: selection.start,
            end: selection.end,
            text: selection.text,
            quads: spike_library::margin::measured_quads(&rects),
            text_right: selection.right,
        });
    }

    fn margin_frame(&self) -> (String, String, u32, bool, f64, f64) {
        let book = self.book_path.clone();
        match self.mode {
            Mode::Pdfium | Mode::Hayro => {
                let count = self.pdf.as_ref().map(|pdf| pdf.sizes.len()).unwrap_or(0);
                if count == 0 {
                    return (book, String::new(), 0, false, 0.0, 0.0);
                }
                let index = self.current_page().min(count - 1);
                let (from, incoming, _) = self.pdf_shift(index);
                let moving = incoming.is_some();
                let Some(shown) = crate::visit::margin_pdf_index(count, index, from, moving) else {
                    return (book, String::new(), 0, false, 0.0, 0.0);
                };
                let (pw, ph) = self
                    .pdf
                    .as_ref()
                    .and_then(|pdf| pdf.sizes.get(shown))
                    .copied()
                    .unwrap_or((0.0, 0.0));
                (
                    book,
                    String::new(),
                    crate::visit::margin_page_number(shown),
                    moving,
                    f64::from(pw),
                    f64::from(ph),
                )
            }
            Mode::WebKit => {
                let spine = self.web.as_ref().map(|web| web.href()).unwrap_or_default();
                let (moving, page_w, page_h) = self
                    .web
                    .as_ref()
                    .map(|web| {
                        let inbox = web.inbox.borrow();
                        (inbox.leaf, inbox.page_w, inbox.page_h)
                    })
                    .unwrap_or((false, 0.0, 0.0));
                let (page_w, page_h) = spike_library::margin::epub_page_size(page_w, page_h);
                (
                    book,
                    spine,
                    (self.epub_page as u32).saturating_add(1),
                    moving,
                    page_w,
                    page_h,
                )
            }
            _ => (book, String::new(), 0, false, 0.0, 0.0),
        }
    }

    fn current_page(&self) -> usize {
        let Some(pdf) = self.pdf.as_ref() else {
            return 0;
        };
        pdf_index_at(self.offset, &pdf.sizes, self.scale.display() as f64)
    }
}

pub(crate) fn pdf_index_at(offset: f64, sizes: &[(f32, f32)], display: f64) -> usize {
    let mut y = 0.0;
    for (index, (_, ph)) in sizes.iter().enumerate() {
        let dh = *ph as f64 * display;
        if offset < y + dh {
            return index;
        }
        y += dh + 16.0;
    }
    0
}

fn pdf_origin_of(product: bool) -> (f64, f64) {
    if product {
        (crate::chrome::SIDE, crate::chrome::TOP)
    } else {
        (24.0, 0.0)
    }
}

impl State {
    fn pdf_origin(&self) -> (f64, f64) {
        pdf_origin_of(self.product)
    }

    fn page_edge_at(&self, x: f64, y: f64, stage_w: f64, stage_h: f64) -> crate::visit::PageEdge {
        if !matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            return crate::visit::PageEdge::None;
        }
        let Some(pdf) = self.pdf.as_ref() else {
            return crate::visit::PageEdge::None;
        };
        if pdf.sizes.is_empty() {
            return crate::visit::PageEdge::None;
        }
        if self.fits_card() {
            let index = self.current_page().min(pdf.sizes.len() - 1);
            let (pw, ph) = pdf.sizes[index];
            let shown = self.presented_at(stage_w, stage_h);
            let frame = crate::form::ReadingFrame {
                card: shown.page,
                margin: shown.margin,
                covers: shown.covers,
            };
            let (rx, ry, mut rw, rh) = spike_report::fitted_pdf_page(
                shown.page.x,
                shown.page.y,
                shown.page.w,
                shown.page.h,
                f64::from(pw),
                f64::from(ph),
            );
            let (rx, spanned) = crate::form::visible_page_span(rx, rw, &frame);
            rw = spanned;
            return crate::visit::page_edge(x, y, rx, ry, rw, rh);
        }
        let display = self.scale.display() as f64;
        let (origin_x, origin_y) = self.pdf_origin();
        let mut top = origin_y - self.offset;
        for (pw, ph) in &pdf.sizes {
            let height = f64::from(*ph) * display;
            let width = f64::from(*pw) * display;
            if y >= top && y <= top + height {
                return crate::visit::page_edge(x, y, origin_x, top, width, height);
            }
            top += height + 16.0;
        }
        crate::visit::PageEdge::None
    }

    fn pdf_link_target(
        &self,
        x: f64,
        y: f64,
        stage_w: f64,
        stage_h: f64,
    ) -> Option<(u32, spike_pdf::LinkHit)> {
        if !self.fits_card() || !matches!(self.mode, Mode::Pdfium | Mode::Hayro) {
            return None;
        }
        let pdf = self.pdf.as_ref()?;
        if pdf.sizes.is_empty() {
            return None;
        }
        let shown = self.presented_at(stage_w, stage_h);
        let frame = crate::form::ReadingFrame {
            card: shown.page,
            margin: shown.margin,
            covers: shown.covers,
        };
        let margin = frame.margin.map(|rect| (rect.x, rect.y, rect.w, rect.h));
        if crate::visit::margin_owns_point(x, y, frame.covers, margin) {
            return None;
        }
        let index = self.current_page().min(pdf.sizes.len() - 1);
        let (from, incoming, shift) = self.pdf_shift(index);
        let page = shown.page;
        let key_forward = self
            .pdf_motion
            .as_ref()
            .map(|motion| motion.slide.forward)
            .unwrap_or(true);
        let forward = crate::visit::neighbor_on_the_right(shift, key_forward);
        let mut painted = Vec::new();
        if let Some(slot) = fitted_link_page(pdf, from, page, 0.0, shift) {
            painted.push(slot);
        }
        if let Some(next) = incoming {
            let step = fitted_link_page(pdf, from, page, 0.0, 0.0)
                .map(|slot| if forward { slot.w } else { -slot.w })
                .unwrap_or(0.0);
            if let Some(slot) = fitted_link_page(pdf, next, page, step, shift) {
                painted.push(slot);
            }
        }
        for slot in painted.into_iter().rev() {
            let links = pdf.doc.page_links(slot.index);
            if let Some(hit) = crate::visit::hit_fitted_page(
                x,
                y,
                slot.x,
                slot.y,
                slot.w,
                slot.h,
                slot.pw,
                slot.ph,
                &links,
                slot.index as u32 + 1,
            ) {
                return Some(hit);
            }
        }
        None
    }
}

struct FittedLinkPage {
    index: usize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    pw: f32,
    ph: f32,
}

fn fitted_link_page(
    pdf: &PdfView,
    index: usize,
    page: crate::form::Rect,
    step: f64,
    shift: f64,
) -> Option<FittedLinkPage> {
    let (pw, ph) = pdf.sizes.get(index).copied()?;
    if pw < 1.0 || ph < 1.0 {
        return None;
    }
    let (x, y, w, h) =
        spike_report::fitted_pdf_page(page.x, page.y, page.w, page.h, f64::from(pw), f64::from(ph));
    Some(FittedLinkPage {
        index,
        x: x - shift + step,
        y,
        w,
        h,
        pw,
        ph,
    })
}

fn locate(
    pdf: &PdfView,
    offset: f64,
    x: f64,
    y: f64,
    display: f64,
    origin_x: f64,
    origin_y: f64,
) -> Option<(usize, f32, f32)> {
    let mut top = origin_y - offset;
    for (index, (pw, ph)) in pdf.sizes.iter().enumerate() {
        let dh = *ph as f64 * display;
        let bottom = top + dh;
        if y >= top && y <= bottom {
            let px = ((x - origin_x) / display) as f32;
            let py = ((y - top) / display) as f32;
            if px >= 0.0 && px <= *pw {
                return Some((index, px, py));
            }
        }
        top = bottom + 16.0;
    }
    None
}

fn elapsed_ms(at: Option<Instant>) -> u32 {
    at.map(|stamp| stamp.elapsed().as_millis().min(u128::from(u32::MAX)) as u32)
        .unwrap_or(0)
}

fn excerpt_window(text: &str, offset: u32) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return String::new();
    }
    let at = (offset as usize).min(units.len() - 1);
    let from = at.saturating_sub(12);
    let to = (at + 36).min(units.len());
    String::from_utf16_lossy(&units[from..to])
}

fn pages_left(left: usize) -> String {
    if left == 1 {
        "1 page left in this chapter".into()
    } else {
        format!("{left} pages left in this chapter")
    }
}

fn scale_key(scale: PixelScale) -> u8 {
    match scale {
        PixelScale::Retina1x => 1,
        PixelScale::Zoom2x => 2,
    }
}

struct Stage {
    view: Retained<StageView>,
    state: Rc<RefCell<State>>,
}

impl Stage {
    fn new(context: ComponentContext) -> Self {
        let mtm = MainThreadMarker::new().expect("main");
        let state = Rc::new(RefCell::new(State {
            context,
            mode: Mode::Pdfium,
            scale: PixelScale::Retina1x,
            offset: 0.0,
            pinch: 0.0,
            drag: None,
            pdf: None,
            book_kind: "pride",
            book_path: String::new(),
            book_gen: 1,
            drag_gen: 0,
            pdf_motion: None,
            pdf_drag: 0.0,
            pdf_glide: None,
            pdf_card: (0.0, 0.0),
            pdf_swipe: crate::visit::Swipe::default(),
            product: false,
            pdf_saved_page: 0,
            pdf_saved_offset: 0,
            page_total: None,
            epub_page: 0,
            anchor: None,
            anchor_page: None,
            note: None,
            selection_token: 0,
            selection_view: 0.0,
            selection_view_h: 0.0,
            selection_gen: 0,
            anchor_payload: String::new(),
            web: None,
            web_pages: 1,
            trim_id: crate::settings::load(&crate::settings::path()),
            declared: None,
            stage_w: 0.0,
            stage_h: 0.0,
            shell_w: 0.0,
            shell_h: 0.0,
            placed: None,
            placed_zoom: 1.0,
            placed_stage_w: 0.0,
            placed_stage_h: 0.0,
            await_hold: false,
            settled: false,
            hold_since: None,
            shown_at: None,
            footer_at: None,
            play_count: 0,
            play_at: None,
            turn_pending: false,
            seen_frames: 0,
            title_settled: false,
            chrome: Rc::new(crate::chrome::Runtime::default()),
            ease_leg: 0,
            ease_started: None,
            ease_flag: false,
            ease_sent: false,
            shelf_leg: 0,
            shelf_started: None,
            shelf_alpha: -1.0,
            find: None,
            find_paint: Vec::new(),
            find_current: None,
            excerpt_key: String::new(),
            excerpt_text: String::new(),
        }));
        let view = mtm.alloc::<StageView>().set_ivars(Ivars {
            state: state.clone(),
        });
        let view: Retained<StageView> = unsafe {
            msg_send![super(view), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0))]
        };
        unsafe { msg_send![&*view, setWantsLayer: true] }
        let product = crate::books::in_app_bundle();
        {
            let mut state = state.borrow_mut();
            state.product = product;
            state.chrome.product.set(product);
            if product {
                state.chrome.lights.set(1.0);
                state.mode = Mode::Empty;
            }
        }
        // An explicit file is decided before the shelf. A bare launch is Full
        // and does not consume `place`. The stored page is for this file and
        // for 继续阅读, not for opening the book on its own.
        let explicit = std::env::var("SUPERREADER_OPEN_PATH").ok().and_then(|raw| {
            let path = raw.trim().to_string();
            if path.is_empty() || !std::path::Path::new(&path).exists() {
                None
            } else {
                Some(path)
            }
        });
        let stored = explicit
            .as_deref()
            .map(|path| crate::settings::resume_page(&crate::settings::path(), path));
        let shelf = crate::launches_full();
        let target =
            crate::launch_target(explicit.is_some(), crate::reader_forcing_launch(), stored);
        if target.engages_shell() {
            crate::chrome::engage_shell();
        }
        match target {
            crate::LaunchTarget::File { page } if product => {
                if let Some(path) = explicit {
                    state.borrow_mut().open_at(&view, &path, page);
                }
            }
            crate::LaunchTarget::Shelf => {
                state.borrow_mut().mode = Mode::Empty;
                state.borrow_mut().publish();
            }
            crate::LaunchTarget::File { .. } | crate::LaunchTarget::Forced => {}
        }
        if !product && !shelf {
            let engine = match std::env::var("SPIKE_ENGINE").as_deref() {
                Ok("hayro") => PdfEngine::Hayro,
                _ => PdfEngine::Pdfium,
            };
            if std::env::var("SPIKE_SCALE").as_deref() == Ok("zoom-2x") {
                state.borrow_mut().scale = PixelScale::Zoom2x;
            }
            state.borrow_mut().mode = if engine == PdfEngine::Hayro {
                Mode::Hayro
            } else {
                Mode::Pdfium
            };
            state.borrow_mut().load_pdf(engine);
            if std::env::var("SPIKE_DRAG").as_deref() == Ok("1") {
                state.borrow_mut().drag_two_lines();
            }
            if std::env::var("SPIKE_MODE").as_deref() == Ok("webkit") {
                let mut state = state.borrow_mut();
                state.mode = Mode::WebKit;
                state.book_kind = book_kind_from_env();
                let path = state.epub_path();
                crate::web::set_book(&path);
                crate::web::cancel_swipe();
                state.open_web(&view, "");
                if let Some(web) = state.web.as_ref() {
                    let href = web.href();
                    crate::web::open_reader(1, 0, &href, false);
                    if let Ok(page) = std::env::var("SPIKE_AT").unwrap_or_default().parse::<u32>() {
                        crate::web::prime_walk(page);
                    }
                }
                state.publish();
            }
            if std::env::var("SPIKE_MODE").as_deref() == Ok("text") {
                state.borrow_mut().mode = Mode::Text;
                state.borrow_mut().book_kind = book_kind_from_env();
                state.borrow_mut().publish();
            }
            if std::env::var("SPIKE_SELECT").as_deref() == Ok("1") {
                state.borrow_mut().select_phrase("Abstract");
            }
        }
        schedule_shell_pump(&*view, 0.05);
        let _: () = unsafe {
            msg_send![
                &*view,
                performSelector: sel!(pollSettings),
                withObject: std::ptr::null::<AnyObject>(),
                afterDelay: 0.4_f64
            ]
        };
        Stage { view, state }
    }
}

impl NativeComponent for Stage {
    fn view(&self) -> Retained<NSView> {
        Retained::into_super(self.view.clone())
    }

    fn set_prop(&mut self, _name: &str, _value: Option<&nsh_core::JsonValue>) {}

    fn command(&mut self, name: &str, args: &nsh_core::JsonValue) {
        if name == "open" {
            let path = args
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    args.get("path")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            self.state.borrow_mut().open_requested(&self.view, &path);
        } else if name == "trim" {
            self.state
                .borrow_mut()
                .apply_trim(args.as_str().unwrap_or(""));
        } else {
            self.state.borrow_mut().command(name, args);
        }
        unsafe { msg_send![&*self.view, setNeedsDisplay: true] }
    }
}

fn add_chapter_item(menu: &AnyObject, target: &AnyObject, title: &str, tag: isize) {
    let Some(item_cls) = AnyClass::get(c"NSMenuItem") else {
        return;
    };
    unsafe {
        let alloc: Allocated<AnyObject> = msg_send![item_cls, alloc];
        let item: Retained<AnyObject> = msg_send![
            alloc,
            initWithTitle: &*NSString::from_str(title),
            action: sel!(pickChapter:),
            keyEquivalent: &*NSString::from_str("")
        ];
        let _: () = msg_send![&*item, setTarget: target];
        let _: () = msg_send![&*item, setTag: tag];
        let _: () = msg_send![menu, addItem: &*item];
    }
}

fn pop_chapters(view: &StageView, titles: &[String], back: bool) {
    let Some(menu_cls) = AnyClass::get(c"NSMenu") else {
        return;
    };
    let Some(app_cls) = AnyClass::get(c"NSApplication") else {
        return;
    };
    let target = unsafe { &*std::ptr::from_ref(view).cast::<AnyObject>() };
    unsafe {
        let menu: Retained<AnyObject> = msg_send![menu_cls, new];
        if back {
            add_chapter_item(&menu, target, "返回", -1);
        }
        for (index, title) in titles.iter().enumerate() {
            add_chapter_item(&menu, target, title, index as isize);
        }
        let app: *mut AnyObject = msg_send![app_cls, sharedApplication];
        if app.is_null() {
            return;
        }
        let event: *mut AnyObject = msg_send![app, currentEvent];
        if event.is_null() {
            return;
        }
        let _: () = msg_send![
            menu_cls,
            popUpContextMenu: &*menu,
            withEvent: event,
            forView: target
        ];
    }
}

fn fill_card(ctx: &CGContext, x: f64, y: f64, w: f64, h: f64, radius: f64, shadow: f64) {
    if shadow > 0.001 {
        let spread = f64::from(crate::tokens::CARD_SPREAD);
        let alpha = f64::from(crate::tokens::CARD_SPREAD_ALPHA) * shadow;
        fill_round(
            ctx,
            x - spread,
            y - spread,
            w + spread * 2.0,
            h + spread * 2.0,
            radius + spread,
            0.0,
            0.0,
            0.0,
            alpha,
        );
    }
    fill_round(ctx, x, y, w, h, radius, 1.0, 1.0, 1.0, 1.0);
}

fn fill_round(
    ctx: &CGContext,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius: f64,
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
) {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    unsafe {
        CGContext::set_rgb_fill_color(Some(ctx), red, green, blue, alpha);
        CGContext::begin_path(Some(ctx));
        CGContext::move_to_point(Some(ctx), x + r, y);
        CGContext::add_arc_to_point(Some(ctx), x + w, y, x + w, y + h, r);
        CGContext::add_arc_to_point(Some(ctx), x + w, y + h, x, y + h, r);
        CGContext::add_arc_to_point(Some(ctx), x, y + h, x, y, r);
        CGContext::add_arc_to_point(Some(ctx), x, y, x + w, y, r);
        CGContext::close_path(Some(ctx));
        CGContext::fill_path(Some(ctx));
    }
}

fn decorate_page(shown: &mut Rgba, index: usize, decoration: &PageDecoration) {
    let notes = if decoration.anchor_page == Some(index as u32) {
        decoration.notes.clone()
    } else {
        Vec::new()
    };
    let marks = decoration
        .paint
        .iter()
        .filter(|paint| paint.page == index)
        .map(|paint| {
            let underline = notes.iter().any(|note| boxes_overlap(&paint.quad, note));
            let current = decoration.current.is_some_and(|(page, start, end)| {
                page == index as u32 + 1 && paint.start == start && paint.end == end
            });
            FindMark {
                quad: paint.quad,
                current,
                underline,
            }
        })
        .collect::<Vec<_>>();
    let (fills, lines): (Vec<_>, Vec<_>) = marks.into_iter().partition(|mark| !mark.underline);
    if !fills.is_empty() {
        paint_find(shown, &fills, decoration.scale);
    }
    if !notes.is_empty() {
        paint_highlight(shown, &notes, decoration.scale);
    }
    if !lines.is_empty() {
        paint_find(shown, &lines, decoration.scale);
    }
}

fn boxes_overlap(a: &spike_report::Quad, b: &spike_report::Quad) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

pub(crate) fn page_offset(sizes: &[(f32, f32)], page: u32, scale: PixelScale) -> f64 {
    let display = scale.display() as f64;
    let index = page.saturating_sub(1) as usize;
    let mut y = 0.0;
    for (i, (_, height)) in sizes.iter().enumerate() {
        if i == index {
            return y;
        }
        y += *height as f64 * display + 16.0;
    }
    0.0
}

fn book_kind_from_env() -> &'static str {
    match std::env::var("SPIKE_BOOK").as_deref() {
        Ok("zhaohua") => "zhaohua",
        Ok("styled") => "styled",
        Ok("publisher") => "publisher",
        Ok("plain-en") => "plain-en",
        Ok("plain-zh") => "plain-zh",
        Ok("fixed") => "fixed",
        _ => "pride",
    }
}

fn anchor_start(anchor: &Anchor) -> Option<u32> {
    match anchor {
        Anchor::Epub { start, .. } => Some(*start),
        Anchor::Pdf { .. } => None,
    }
}

fn anchor_end(anchor: &Anchor) -> Option<u32> {
    match anchor {
        Anchor::Epub { end, .. } => Some(*end),
        Anchor::Pdf { .. } => None,
    }
}

fn hold_replaced_frame(web: &crate::web::WebChapter, width: f64, height: f64, zoom: f64) {
    let mut inbox = web.inbox.borrow_mut();
    let previous_w = inbox.page_w;
    let previous_h = inbox.page_h;
    let previous_zoom = inbox.layout_zoom;
    if spike_library::margin::layout_moved(
        previous_w,
        previous_h,
        previous_zoom,
        width,
        height,
        zoom,
    ) {
        inbox.hold_layout();
    }
    inbox.page_w = width;
    inbox.page_h = height;
    inbox.layout_zoom = zoom;
}

fn layout_target(placed_width: f64, zoom: f64) -> f64 {
    if zoom.is_finite() && zoom > 0.01 {
        placed_width / zoom
    } else {
        placed_width
    }
}

/// `performSelector:afterDelay:` waits in the default mode only. Common modes
/// include the tracking mode a window drag runs in, so a pump still fires
/// while the mouse is down.
fn schedule_on_common_modes(view: &StageView, selector: Sel, delay: f64) {
    let modes = NSArray::from_slice(&[unsafe { NSRunLoopCommonModes }]);
    let _: () = unsafe {
        msg_send![
            view,
            performSelector: selector,
            withObject: std::ptr::null::<AnyObject>(),
            afterDelay: delay,
            inModes: &*modes,
        ]
    };
}

fn schedule_shell_pump(view: &StageView, delay: f64) {
    schedule_on_common_modes(view, sel!(pumpLights), delay);
}

fn schedule_chrome_pump(view: &StageView, delay: f64) {
    schedule_on_common_modes(view, sel!(pumpChrome), delay);
}

/// One shared key, so a layout callback and the pump do not publish the same
/// size twice. The layout callback is what AppKit delivers during a drag.
fn publish_shell_if_resized(state: &Rc<RefCell<State>>, width: f64, height: f64, source: &str) {
    let key = {
        let mut state = state.borrow_mut();
        state.shell_w = width;
        state.shell_h = height;
        shell_publish_key(
            state.shell_w,
            state.shell_h,
            state.chrome.bar_w.get(),
            state.chrome.lights_visible.get(),
        )
    };
    thread_local! {
        static LAST: std::cell::Cell<Option<(i32, i32, i32, i32)>> =
            const { std::cell::Cell::new(None) };
    }
    let effect = LAST.with(|last| {
        let effect = publish_key_effect(last.get(), key);
        if effect.publish {
            last.set(key);
        }
        effect
    });
    trace_resize(source, width, height, effect.publish);
    if effect.resize {
        state.borrow().chrome.resize_chrome();
    }
    if effect.publish {
        state.borrow_mut().publish();
    }
}

pub(crate) fn trace_resize(source: &str, width: f64, height: f64, published: bool) {
    if std::env::var("SPIKE_RESIZE_TRACE").ok().as_deref() != Some("1") {
        return;
    }
    let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return;
    };
    let line = format!(
        "{{\"t\":{:.4},\"src\":\"{source}\",\"w\":{width:.1},\"h\":{height:.1},\"published\":{published}}}\n",
        elapsed.as_secs_f64()
    );
    let path = std::path::Path::new("/tmp/superreader-spike/resize-trace.jsonl");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = file.write_all(line.as_bytes());
    }
}

/// The shell status event is skipped when this key is unchanged. Lights are
/// part of it: exit restores the old window size before the buttons are
/// visible again, and a size-only key then never publishes the shown lights.
/// A lights-only change still publishes. It is this window's own alpha crossing
/// half, not a new size, so it does not drop the hide that is already running.
pub(crate) fn publish_key_effect(
    previous: Option<(i32, i32, i32, i32)>,
    next: Option<(i32, i32, i32, i32)>,
) -> PublishEffect {
    match (previous, next) {
        (previous, Some(next)) if previous != Some(next) => PublishEffect {
            publish: true,
            resize: previous.map(|(width, height, bar, _)| (width, height, bar))
                != Some((next.0, next.1, next.2)),
        },
        _ => PublishEffect {
            publish: false,
            resize: false,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PublishEffect {
    pub publish: bool,
    pub resize: bool,
}

pub(crate) fn shell_publish_key(
    shell_w: f64,
    shell_h: f64,
    bar_w: f64,
    lights_visible: bool,
) -> Option<(i32, i32, i32, i32)> {
    let key = (
        shell_w.round() as i32,
        shell_h.round() as i32,
        bar_w.round() as i32,
        i32::from(lights_visible),
    );
    if key.0 < 2 || key.1 < 2 {
        None
    } else {
        Some(key)
    }
}

fn ease_tick_due(trip_moving: bool, trip_was: bool, shelf_was: bool) -> bool {
    trip_moving || trip_was || shelf_was
}

pub(crate) struct EasePump {
    pub emit: bool,
    pub trip_moving: bool,
    pub shelf_moving: bool,
    pub trip: crate::ease::Trip,
}

/// The production clock `sync_ease` runs. `trip_was` is the trip before
/// `ease::advance`, and the emit flag stays on for the landing call.
pub(crate) fn pump_ease(trip_elapsed: u32, shelf_elapsed: u32) -> EasePump {
    let trip = crate::ease::current();
    let trip_was = trip.moving();
    let trip = if trip_was {
        crate::ease::advance(trip_elapsed)
    } else {
        trip
    };
    let shelf_was = crate::shelf_pass::moving();
    if shelf_was {
        crate::shelf_pass::advance(shelf_elapsed);
    }
    EasePump {
        emit: ease_tick_due(trip.moving(), trip_was, shelf_was),
        trip_moving: trip.moving(),
        shelf_moving: crate::shelf_pass::moving(),
        trip,
    }
}

#[cfg(test)]
mod tests {
    use super::{layout_target, pump_ease, shell_publish_key};

    #[test]
    fn the_immersive_landing_tick_is_emitted_after_the_trip_stops() {
        crate::shelf_pass::reset();
        crate::ease::settle(false, false);
        crate::ease::retarget(true, true);
        assert!(crate::ease::current().moving());
        let early = pump_ease(0, 0);
        assert!(early.emit);
        assert!(early.trip_moving);
        assert!(crate::ease::current().moving());
        let landed = pump_ease(10_000, 0);
        assert!(landed.emit);
        assert!(!landed.trip_moving);
        assert!(!crate::ease::current().moving());
        let idle = pump_ease(10_000, 0);
        assert!(!idle.emit);
        assert!(!idle.trip_moving);

        crate::shelf_pass::reset();
        crate::ease::settle(false, false);
        crate::shelf_pass::intend_reading("a.pdf", None);
        assert!(crate::shelf_pass::moving());
        assert!(!crate::ease::current().moving());
        let shelf_early = pump_ease(0, 0);
        assert!(shelf_early.emit);
        assert!(shelf_early.shelf_moving);
        let shelf_landed = pump_ease(0, 10_000);
        assert!(shelf_landed.emit);
        assert!(!shelf_landed.shelf_moving);
        assert!(!crate::shelf_pass::moving());
        let shelf_idle = pump_ease(0, 10_000);
        assert!(!shelf_idle.emit);
        crate::shelf_pass::reset();
        crate::ease::settle(false, false);
    }

    #[test]
    fn a_smaller_placement_is_the_layout_target() {
        assert_eq!(layout_target(400.0, 1.0), 400.0);
        assert_eq!(layout_target(800.0, 2.0), 400.0);
        assert!(layout_target(400.0, 1.0) < 534.0);
    }

    #[test]
    fn live_resize_publishes_from_layout_in_common_modes() {
        let source = include_str!("stage.rs");
        let layout = source
            .split("fn layout_view")
            .nth(1)
            .and_then(|rest| rest.split("fn scroll_wheel").next())
            .unwrap_or("");
        assert!(
            layout.contains("publish_shell_if_resized"),
            "layout() has to publish the content size while a drag is tracking"
        );
        let pump = source
            .split("fn pump_lights")
            .nth(1)
            .and_then(|rest| rest.split("fn poll_settings").next())
            .unwrap_or("");
        assert!(
            pump.contains("NSRunLoopCommonModes") || pump.contains("schedule_shell_pump"),
            "the lights pump has to be scheduled in the tracking run-loop mode"
        );
        let schedule = source
            .split("fn schedule_on_common_modes")
            .nth(1)
            .unwrap_or("");
        assert!(
            schedule.contains("NSRunLoopCommonModes"),
            "a pump must use common modes, not afterDelay alone"
        );
        let draw = source
            .split("fn draw(")
            .nth(1)
            .and_then(|rest| rest.split("fn scroll(").next())
            .unwrap_or("");
        assert!(
            draw.contains("schedule_chrome_pump"),
            "the PDF slide pump has to run while a window drag is tracking"
        );
        assert!(
            !draw.contains("sel!(pumpChrome)"),
            "pumpChrome must not wait in the default run-loop mode"
        );
        let finish = source
            .split("fn finish_pdf_motion")
            .nth(1)
            .and_then(|rest| rest.split("fn start_pdf_slide").next())
            .unwrap_or("");
        assert!(finish.contains("pdf_turn"));
        assert!(finish.contains("note_page"));
        let chapter = source
            .split("fn apply_chapter")
            .nth(1)
            .and_then(|rest| rest.split("fn return_saved_place").next())
            .unwrap_or("");
        let choice = chapter.find("apply_contents_choice").unwrap_or(usize::MAX);
        let page = chapter.find("ChapterMove::Page").unwrap_or(0);
        assert!(
            choice < page,
            "a contents choice drops the unfinished PDF slide before that page is shown"
        );
        assert!(chapter.contains("pdf_glide"));
        let edge = source
            .split("fn page_edge_at")
            .nth(1)
            .and_then(|rest| rest.split("fn locate").next())
            .unwrap_or("");
        assert!(
            edge.contains("visible_page_span"),
            "a click uses the page that remains beside the cover"
        );
    }

    #[test]
    fn a_light_change_republishes_the_restored_window() {
        let hidden = shell_publish_key(1440.0, 855.0, 78.0, false);
        let shown = shell_publish_key(1440.0, 855.0, 78.0, true);
        assert_eq!(hidden, Some((1440, 855, 78, 0)));
        assert_eq!(shown, Some((1440, 855, 78, 1)));
        assert_ne!(hidden, shown);
        assert_eq!(shell_publish_key(1.0, 855.0, 78.0, true), None);
    }
}

fn current_context() -> Option<Retained<CGContext>> {
    unsafe {
        let current: Option<Retained<objc2_app_kit::NSGraphicsContext>> =
            msg_send![objc2::class!(NSGraphicsContext), currentContext];
        current.and_then(|g| -> Option<Retained<CGContext>> { msg_send![&*g, CGContext] })
    }
}

pub fn register(components: &Components) {
    components.register(
        "reader.stage",
        Rc::new(|context| Box::new(Stage::new(context))),
    );
}
