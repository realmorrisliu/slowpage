use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use nsh_platform_macos::objc2::{
    define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSButton, NSColor, NSColorSpace, NSEvent, NSEventMask, NSFont, NSFontWeightRegular, NSImage,
    NSImageAlignment, NSImageScaling, NSImageSymbolConfiguration, NSImageView, NSLineBreakMode,
    NSScreen, NSTextField, NSTitlebarSeparatorStyle, NSToolbar, NSToolbarDelegate,
    NSToolbarDisplayMode, NSToolbarFlexibleSpaceItemIdentifier, NSToolbarItem,
    NSUserInterfaceItemIdentification, NSView, NSWindow, NSWindowButton,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};

use crate::icons;

pub const SIDE: f64 = 52.0;
pub const TOP: f64 = crate::tokens::CHROME_H as f64;
pub const FIT: f64 = 0.88;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub window_w: f64,
    pub window_h: f64,
    pub page_w: f64,
    pub page_h: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct PageBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub stage_w: f64,
    pub stage_h: f64,
    pub ratio_w: f64,
    pub ratio_h: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct TrafficLights {
    pub close_min_x: f64,
    pub close_max_x: f64,
    pub close_min_y: f64,
    pub close_max_y: f64,
    pub mini_min_x: f64,
    pub mini_max_x: f64,
    pub mini_min_y: f64,
    pub mini_max_y: f64,
    pub zoom_min_x: f64,
    pub zoom_max_x: f64,
    pub zoom_min_y: f64,
    pub zoom_max_y: f64,
    pub width: f64,
}

pub struct Runtime {
    settled: Cell<bool>,
    stable: Cell<u8>,
    attempts: Cell<u8>,
    shown: Cell<bool>,
    pub full: Cell<bool>,
    pub product: Cell<bool>,
    fullscreen_sent: Cell<bool>,
    fullscreen_exit_sent: Cell<bool>,
    pub lights: Cell<f64>,
    ratio_key: Cell<(i32, i32)>,
    page: Cell<PageBox>,
    title: RefCell<String>,
    styled: Cell<bool>,
    entering: Cell<bool>,
    field: RefCell<Option<Retained<NSTextField>>>,
    hover_title: RefCell<Option<Retained<NSTextField>>>,
    contents_button: RefCell<Option<Retained<NSButton>>>,
    contents_bar: RefCell<Option<Retained<ContentsBar>>>,
    icon_gap: Cell<f64>,
    icon_dy: Cell<f64>,
    pub design_visible: Cell<(f64, f64)>,
    pub bar_w: Cell<f64>,
    pub lights_visible: Cell<bool>,
    pub content_top: Cell<f64>,
    pub stage_h: Cell<f64>,
    pub bar_dirty: Cell<bool>,
    bar_seen: Cell<f64>,
    bar_stable: Cell<u8>,
    frame_key: Cell<(i32, i32)>,
    toolbar_style: Cell<isize>,
    style_mask: Cell<usize>,
    pub bar_gen: Cell<u32>,
    pub immersive: Cell<bool>,
    pub reading: Cell<bool>,
    bar_fullscreen_sent: Cell<bool>,
    pub laying_out: Cell<bool>,
    traffic: RefCell<Option<TrafficLights>>,
    chrome_fade: Cell<crate::reading::ChromeFade>,
    chrome_gen: Cell<u64>,
    chrome_cut: Cell<bool>,
    chrome_tick: Cell<Option<Instant>>,
    sent_lights: Cell<f64>,
    title_opacity: Cell<f64>,
    chrome_shared: Cell<bool>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            settled: Cell::new(false),
            stable: Cell::new(0),
            attempts: Cell::new(0),
            shown: Cell::new(false),
            full: Cell::new(false),
            product: Cell::new(false),
            fullscreen_sent: Cell::new(false),
            fullscreen_exit_sent: Cell::new(false),
            lights: Cell::new(0.0),
            ratio_key: Cell::new((0, 0)),
            page: Cell::new(PageBox {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                stage_w: 0.0,
                stage_h: 0.0,
                ratio_w: 0.0,
                ratio_h: 0.0,
            }),
            title: RefCell::new(String::new()),
            styled: Cell::new(false),
            entering: Cell::new(false),
            field: RefCell::new(None),
            hover_title: RefCell::new(None),
            contents_button: RefCell::new(None),
            contents_bar: RefCell::new(None),
            icon_gap: Cell::new(f64::NAN),
            icon_dy: Cell::new(f64::NAN),
            design_visible: Cell::new((0.0, 0.0)),
            bar_w: Cell::new(0.0),
            lights_visible: Cell::new(false),
            content_top: Cell::new(0.0),
            stage_h: Cell::new(0.0),
            bar_dirty: Cell::new(true),
            bar_seen: Cell::new(0.0),
            bar_stable: Cell::new(0),
            frame_key: Cell::new((0, 0)),
            toolbar_style: Cell::new(-2),
            style_mask: Cell::new(usize::MAX),
            bar_gen: Cell::new(0),
            immersive: Cell::new(false),
            reading: Cell::new(false),
            bar_fullscreen_sent: Cell::new(false),
            laying_out: Cell::new(false),
            traffic: RefCell::new(None),
            chrome_fade: Cell::new(crate::reading::ChromeFade::hidden()),
            chrome_gen: Cell::new(0),
            chrome_cut: Cell::new(false),
            chrome_tick: Cell::new(None),
            sent_lights: Cell::new(f64::NAN),
            title_opacity: Cell::new(1.0),
            chrome_shared: Cell::new(false),
        }
    }
}

impl Runtime {
    pub fn set_title(&self, title: &str) {
        *self.title.borrow_mut() = title.to_string();
    }

    pub fn cut_chrome(&self) {
        self.chrome_cut.set(true);
        self.chrome_gen.set(self.chrome_gen.get().wrapping_add(1));
        self.lights.set(0.0);
        self.sent_lights.set(f64::NAN);
    }

    pub fn resize_chrome(&self) {
        self.chrome_gen.set(self.chrome_gen.get().wrapping_add(1));
    }

    pub fn lights_changed(&self, alpha: f64) -> bool {
        let prev = self.sent_lights.get();
        if prev.is_finite() && (prev - alpha).abs() < 0.001 {
            return false;
        }
        self.sent_lights.set(alpha);
        true
    }

    pub fn note_chrome_status(&self, title_opacity: f64, chrome_shared: bool) {
        self.title_opacity.set(title_opacity);
        self.chrome_shared.set(chrome_shared);
    }

    pub fn mark_bar_dirty(&self) {
        self.bar_dirty.set(true);
        self.bar_stable.set(0);
        self.bar_seen.set(0.0);
        self.frame_key.set((0, 0));
    }
}

/// Width that leaves the same gap on both sides of the traffic lights.
/// `close_min_x` and `zoom_max_x` are in window coordinates.
pub fn centered_bar_width(close_min_x: f64, zoom_max_x: f64) -> f64 {
    zoom_max_x + close_min_x
}

/// Horizontal origin of the reader page. The side mats are equal. The
/// traffic-light row is not a column, and it does not push the page right.
pub fn reader_page_x(left: f64, ix: f64) -> f64 {
    left + ix
}

/// Where a page would sit if it were kept clear of a left bar. The reader
/// does not use this. The test keeps it to show the shift that was removed.
#[cfg(test)]
pub fn content_page_x(stage_w: f64, page_w: f64, bar: f64) -> Option<f64> {
    if bar <= 1.0 || page_w <= 0.0 || stage_w <= bar + 1.0 || page_w > stage_w - bar + 0.5 {
        return None;
    }
    Some(bar + (stage_w - bar - page_w) / 2.0)
}

pub fn title_face(title: &str) -> &'static str {
    if title
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    {
        "Songti SC"
    } else {
        "Iowan Old Style"
    }
}

pub fn fit(ratio_w: f64, ratio_h: f64, visible_w: f64, visible_h: f64) -> Frame {
    let ratio = if ratio_h <= 0.0 {
        1.0
    } else {
        (ratio_w / ratio_h).max(0.05)
    };
    let max_w = (visible_w * FIT).max(1.0);
    let max_h = (visible_h * FIT).max(1.0);
    let from_h = max_h - TOP;
    let from_w = (max_w - SIDE * 2.0) / ratio;
    let page_h = from_h.min(from_w).max(1.0);
    let page_w = page_h * ratio;
    Frame {
        window_w: page_w + SIDE * 2.0,
        window_h: page_h + TOP,
        page_w,
        page_h,
    }
}

pub fn content_box(stage_w: f64, stage_h: f64) -> (f64, f64, f64, f64) {
    let box_w = (stage_w - SIDE * 2.0).max(1.0);
    let box_h = (stage_h - TOP).max(1.0);
    (box_w, box_h, SIDE, TOP)
}

pub fn reading_page(
    ratio_w: f64,
    ratio_h: f64,
    full: bool,
    remembered: &Cell<(f64, f64)>,
) -> (f64, f64) {
    let Some(mtm) = MainThreadMarker::new() else {
        return (1.0, 1.0);
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return (1.0, 1.0);
    };
    let visible = screen.visibleFrame().size;
    let saved = remembered.get();
    if !full && (saved.0 < 1.0 || visible.height <= saved.1 + 1.0) {
        remembered.set((visible.width, visible.height));
    }
    let saved = remembered.get();
    let (width, height) = if saved.0 > 1.0 {
        saved
    } else {
        (visible.width, visible.height)
    };
    let frame = fit(ratio_w, ratio_h, width, height);
    (frame.page_w, frame.page_h)
}

pub fn should_drag_margin(x: f64, y: f64, placed: Option<(i32, i32, i32, i32)>) -> bool {
    let Some((left, top, width, height)) = placed else {
        return false;
    };
    let right = f64::from(left + width);
    let bottom = f64::from(top + height);
    x < f64::from(left) || y < f64::from(top) || x > right || y > bottom
}

static SHELL: AtomicBool = AtomicBool::new(false);

pub fn engage_shell() {
    SHELL.store(true, Ordering::Relaxed);
}

pub fn shell_on() -> bool {
    SHELL.load(Ordering::Relaxed)
}

pub fn shell_opening_size() -> (i32, i32) {
    if let Some((width, height)) = spike_window() {
        return (width.round() as i32, height.round() as i32);
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return (1280, 800);
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return (1280, 800);
    };
    let visible = screen.visibleFrame();
    let width = (visible.size.width * 0.86).round() as i32;
    let height = (visible.size.height * 0.86).round() as i32;
    (width.max(960), height.max(640))
}

pub fn opening_size() -> (i32, i32) {
    if let Some((width, height)) = spike_window() {
        return (width.round() as i32, height.round() as i32);
    }
    let id = crate::settings::load(&crate::settings::path());
    let item = crate::reading::trim(&id);
    let Some(mtm) = MainThreadMarker::new() else {
        return (700, 1040);
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return (700, 1040);
    };
    let visible = screen.visibleFrame();
    let frame = fit(
        f64::from(item.width_mm),
        f64::from(item.height_mm),
        visible.size.width,
        visible.size.height,
    );
    (frame.window_w.round() as i32, frame.window_h.round() as i32)
}

struct Release<'a>(&'a Cell<bool>);

impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

pub fn sync(window: &NSWindow, rt: &Runtime, page: PageBox) {
    if rt.entering.replace(true) {
        return;
    }
    let _release = Release(&rt.entering);
    rt.page.set(page);
    // `set_level` replaces the collection behavior with `MoveToActiveSpace`.
    // The green button and `toggleFullScreen:` both no-op without
    // `FullScreenPrimary`, so put the bit back on every sync, not only the
    // first style.
    keep_fullscreen_primary(window);
    note_blur(window);
    style_window(window, rt);
    // The shelf and the EPUB page share this monitor. A direct file open
    // never engages the shelf, and the page script no longer handles the wheel.
    if crate::visit::arms_page_wheel(shell_on()) {
        install_shelf_wheel();
    }
    window.setColorSpace(Some(&NSColorSpace::sRGBColorSpace()));
    note_immersive(rt);
    settle_bar(window, rt);
    let fullscreen = window.styleMask().contains(NSWindowStyleMask::FullScreen);
    // The pin branch below may mark the window shown. Fullscreen has to see
    // the previous sync, so the toggle is not in the same turn as orderFront.
    let armed = rt.shown.get();
    if hold_pinned_frame(
        fullscreen,
        rt.fullscreen_sent.get(),
        rt.fullscreen_exit_sent.get(),
    ) && !rt.laying_out.get()
    {
        if let Some((width, height)) = pinned_frame() {
            if let Some(screen) = window
                .screen()
                .or_else(|| MainThreadMarker::new().and_then(NSScreen::mainScreen))
            {
                let frame = Frame {
                    window_w: width,
                    window_h: height,
                    page_w: page.w,
                    page_h: page.h,
                };
                apply_frame(window, &screen, &frame, false, !rt.shown.get());
                rt.shown.set(true);
                rt.settled.set(true);
            }
        }
    }
    // Full screen changes the window frame without resizing the content view.
    // The stage then stays at the old height and both panels stop short.
    // Fitting from inside layout re-enters AppKit, so only the pump does it.
    // A live resize is already changing the frame; fitting there fights the
    // drag and can stall the tracking loop. Enter waits until a later turn
    // so the published shelf is the fitted one, not the 32pt-short first frame.
    let was_fitted = content_is_fitted(window);
    if !rt.laying_out.get() && !window.inLiveResize() {
        fit_full_content(window);
    }
    let stable_fit = was_fitted && content_is_fitted(window);
    note_fullscreen_hook(window, rt);
    request_fullscreen(window, rt, fullscreen, armed, stable_fit);
    if fullscreen {
        if shell_on() && rt.reading.get() && !flag("SPIKE_LIGHTS") {
            // The shell covers the screen, so AppKit's fullscreen title bar
            // never receives the hover. The buttons stay on the same content
            // coordinates and fade with the shared reading alpha.
            let alpha = reading_chrome_alpha(window, rt);
            place_fullscreen_lights(window, alpha);
            rt.lights.set(alpha);
            remember_shared(rt, alpha);
            clear_shell_chrome(window);
            place_contents_button(window, rt, alpha);
        } else {
            let shown = hover_target(window);
            if shell_on() {
                place_fullscreen_lights(window, if shown > 0.5 { 1.0 } else { 0.0 });
                rt.lights.set(shown);
                clear_shell_chrome(window);
                if rt.reading.get() {
                    place_contents_button(window, rt, if shown > 0.5 { 1.0 } else { 0.0 });
                }
            } else {
                paint_lights(window, 1.0);
                rt.lights.set(1.0);
                place_contents(window, rt, shown);
                place_title(window, rt, shown);
            }
        }
        rt.full.set(true);
        write_json(window, rt, true);
        return;
    }
    // layout() publishes the new style mask before sync, so full is already
    // false on the exit frame. Put any reparented buttons back whenever this
    // window is not fullscreen. An empty home list is a no-op.
    restore_fullscreen_lights();
    if rt.full.replace(false) {
        rt.settled.set(false);
        rt.stable.set(0);
        rt.attempts.set(0);
    }
    let key = (page.ratio_w.round() as i32, page.ratio_h.round() as i32);
    if key.0 > 0 && key.1 > 0 && rt.ratio_key.get() != key {
        rt.ratio_key.set(key);
        rt.settled.set(false);
        rt.stable.set(0);
        rt.attempts.set(0);
    }
    if !shell_on()
        && pinned_frame().is_none()
        && !rt.settled.get()
        && page.ratio_w > 0.0
        && page.ratio_h > 0.0
    {
        if let Some(screen) = window
            .screen()
            .or_else(|| MainThreadMarker::new().and_then(NSScreen::mainScreen))
        {
            let visible = screen.visibleFrame();
            let frame = if let Some((width, height)) = spike_window() {
                Frame {
                    window_w: width,
                    window_h: height,
                    page_w: (width - SIDE * 2.0).max(1.0),
                    page_h: (height - TOP).max(1.0),
                }
            } else {
                fit(
                    page.ratio_w,
                    page.ratio_h,
                    visible.size.width,
                    visible.size.height,
                )
            };
            let place = !rt.shown.get();
            let animate = rt.shown.get();
            apply_frame(window, &screen, &frame, animate, place);
            rt.shown.set(true);
            let got = window.frame().size;
            let close = (got.width - frame.window_w).abs() < 3.0
                && (got.height - frame.window_h).abs() < 3.0;
            let tries = rt.attempts.get().saturating_add(1);
            rt.attempts.set(tries);
            if close {
                let count = rt.stable.get().saturating_add(1);
                rt.stable.set(count);
                if count >= 3 || tries >= 12 {
                    rt.settled.set(true);
                }
            } else if tries >= 12 {
                rt.settled.set(true);
            } else {
                rt.stable.set(0);
            }
        }
    }
    // A shell reading window shares one alpha with the book title. The
    // library, SPIKE_LIGHTS, and a product window that is not reading keep
    // the lights on. The dev spike still fades toward the pointer.
    let next = if shell_on() && rt.reading.get() && !flag("SPIKE_LIGHTS") {
        reading_chrome_alpha(window, rt)
    } else if shell_on() || flag("SPIKE_LIGHTS") || rt.product.get() {
        1.0
    } else {
        approach(rt.lights.get(), hover_target(window))
    };
    rt.lights.set(next);
    remember_shared(rt, next);
    paint_lights(window, next);
    if shell_on() {
        clear_shell_chrome(window);
        if rt.reading.get() {
            place_contents_button(window, rt, next);
        } else if let Some(button) = rt.contents_button.borrow().clone() {
            button.setHidden(true);
        }
    } else {
        place_contents(window, rt, next);
        place_title(window, rt, next);
    }
    // A shell window opens at SPIKE_WINDOW. The reader settle loop does not
    // run while the shell is on, and the fullscreen flag waits for settled.
    if shell_on() && !rt.settled.get() {
        rt.settled.set(true);
    }
    if shell_on() && window.isVisible() {
        rt.shown.set(true);
    }
    // After a fullscreen exit the buttons are back in the title bar. AppKit
    // then lays the reader toolbar out and can leave the zoom button on the
    // system row while close and minimize sit in the 48pt bar. Align to the
    // shared center, for the reader and the shelf, once that restore is done.
    align_traffic_centers(window);
    // settle_bar stored the buttons before this nudge. Write the frames
    // the buttons actually have, or the green one still reads 8pt high.
    if let Some(measured) = measure_traffic_bar(window) {
        *rt.traffic.borrow_mut() = Some(measured);
    }
    write_json(window, rt, false);
}

pub fn paint_paper_layer(view: &AnyObject) {
    unsafe {
        let _: () = msg_send![view, setWantsLayer: true];
        let layer: Retained<AnyObject> = msg_send![view, layer];
        let (red, green, blue) = crate::reading::paper_rgb();
        let color = objc2_core_graphics::CGColor::new_srgb(red, green, blue, 1.0);
        let _: () = msg_send![&layer, setBackgroundColor: &*color];
    }
}

fn keep_fullscreen_primary(window: &NSWindow) {
    let current = window.collectionBehavior();
    if !current.contains(NSWindowCollectionBehavior::FullScreenPrimary) {
        window.setCollectionBehavior(current | NSWindowCollectionBehavior::FullScreenPrimary);
    }
}

fn style_window(window: &NSWindow, rt: &Runtime) {
    if rt.styled.replace(true) {
        return;
    }
    apply_appearance(window);
    let mask = window.styleMask();
    if !mask.contains(NSWindowStyleMask::FullSizeContentView) {
        window.setStyleMask(mask.union(NSWindowStyleMask::FullSizeContentView));
    }
    window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
    window.setMovableByWindowBackground(true);
    keep_fullscreen_primary(window);
    window.makeKeyAndOrderFront(None);
    expose_window(window);
    let mtm = MainThreadMarker::from(&*window);
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    unsafe {
        let _: () = msg_send![&*app, activateIgnoringOtherApps: true];
    }
    rt.mark_bar_dirty();
}

/// How far each window-space center must move to share one vertical center.
/// The shared center is the median, so the button AppKit left on a different
/// row moves to the other two, and a row that already agrees is not shifted.
pub(crate) fn traffic_center_deltas(centers: &[f64]) -> Vec<f64> {
    if centers.len() < 2 || centers.iter().any(|center| !center.is_finite()) {
        return vec![0.0; centers.len()];
    }
    let mut ordered: Vec<f64> = centers.to_vec();
    ordered.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let shared = ordered[ordered.len() / 2];
    centers.iter().map(|center| shared - center).collect()
}

/// The three system buttons, in window coordinates, onto one center.
/// A button outside the title-bar band is left where it is: fullscreen parks
/// one there, and a fixed green-only offset would not survive the next layout.
fn align_traffic_centers(window: &NSWindow) {
    if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return;
    }
    let top = window.frame().size.height;
    if !top.is_finite() || top < TOP {
        return;
    }
    let band = f64::from(crate::tokens::CHROME_H) + 16.0;
    let mut buttons = Vec::new();
    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let Some(button) = window.standardWindowButton(kind) else {
            continue;
        };
        let frame = button_window_frame(&button);
        if frame.size.height < 1.0 || frame.size.width < 1.0 {
            continue;
        }
        let mid = frame.origin.y + frame.size.height / 2.0;
        let from_top = top - mid;
        if !from_top.is_finite() || !(-1.0..=band).contains(&from_top) {
            continue;
        }
        buttons.push((button, mid));
    }
    if buttons.len() < 2 {
        return;
    }
    let deltas = traffic_center_deltas(&buttons.iter().map(|(_, mid)| *mid).collect::<Vec<_>>());
    for ((button, _), delta) in buttons.iter().zip(deltas) {
        if !delta.is_finite() || delta.abs() <= 0.5 {
            continue;
        }
        let view = view_of(button);
        let Some(parent) = (unsafe { view.superview() }) else {
            continue;
        };
        let frame = button_window_frame(button);
        let shifted = NSPoint::new(frame.origin.x, frame.origin.y + delta);
        let local: NSPoint = unsafe {
            msg_send![
                &*parent,
                convertPoint: shifted,
                fromView: std::ptr::null::<NSView>()
            ]
        };
        button.setFrameOrigin(local);
    }
}

thread_local! {
    static LIGHT_HOME: RefCell<Vec<(Retained<NSButton>, Retained<NSView>, NSRect)>> =
        const { RefCell::new(Vec::new()) };
}

fn view_of(button: &NSButton) -> &NSView {
    let view: &NSView = unsafe { &*std::ptr::from_ref(button).cast::<NSView>() };
    view
}

fn place_fullscreen_lights(window: &NSWindow, alpha: f64) {
    let Some(content) = window.contentView() else {
        return;
    };
    let height = content.frame().size.height;
    for (index, kind) in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .into_iter()
    .enumerate()
    {
        let Some(button) = window.standardWindowButton(kind) else {
            continue;
        };
        let view = view_of(&button);
        if alpha < 0.02 {
            unsafe {
                let _: () = msg_send![&*button, setAlphaValue: 0.0_f64];
                let _: () = msg_send![&*button, setHidden: true];
            }
            continue;
        }
        LIGHT_HOME.with(|slot| {
            let mut slot = slot.borrow_mut();
            let known = slot
                .iter()
                .any(|(saved, _, _)| std::ptr::eq(saved.as_ref(), &*button));
            if !known {
                if let Some(parent) = unsafe { view.superview() } {
                    if !std::ptr::eq::<NSView>(&*parent, &*content) {
                        // The frame is in the title-bar view. Reparenting
                        // keeps it, so the exit path has to put this back.
                        slot.push((button.clone(), parent, view.frame()));
                    }
                }
            }
        });
        content.addSubview(view);
        let x = 9.0 + index as f64 * 22.0;
        let button_h = 14.0;
        let y = height - (f64::from(crate::tokens::chrome_center()) + button_h / 2.0);
        view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(14.0, 14.0)));
        unsafe {
            let _: () = msg_send![&*button, setHidden: false];
            let _: () = msg_send![&*button, setAlphaValue: alpha];
            let _: () = msg_send![&*button, setEnabled: true];
        }
    }
}

fn restore_fullscreen_lights() {
    LIGHT_HOME.with(|slot| {
        for (button, parent, frame) in slot.borrow_mut().drain(..) {
            let view = view_of(&button);
            parent.addSubview(view);
            view.setFrame(frame);
        }
    });
}

/// A full-size content view fills the window frame. A child that kept the
/// previous frame is the unused band under the shelf and the chat.
fn fit_full_content(window: &NSWindow) {
    if !window
        .styleMask()
        .contains(NSWindowStyleMask::FullSizeContentView)
    {
        return;
    }
    let Some(content) = window.contentView() else {
        return;
    };
    let frame = window.frame();
    let current = content.frame();
    let before = view_box(Some(&content));
    if content_needs_fit(
        frame.size.width as f64,
        frame.size.height as f64,
        before.0,
        before.1,
        before.2,
        before.3,
    ) {
        note_fit("content", &frame, current);
        content.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(frame.size.width, frame.size.height),
        ));
    }
    let Some(content) = window.contentView() else {
        return;
    };
    let bounds = content.bounds();
    let Some(child) = filling_child(&content) else {
        return;
    };
    let child_box = view_box(Some(&child));
    if child_needs_fit(
        bounds.size.width as f64,
        bounds.size.height as f64,
        child_box.0,
        child_box.1,
        child_box.2,
        child_box.3,
    ) {
        note_fit("child", &bounds, child.frame());
        child.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(bounds.size.width, bounds.size.height),
        ));
    }
}

pub fn content_size(window: &NSWindow) -> Option<(f64, f64)> {
    let view = window.contentView()?;
    let size = view.bounds().size;
    let width = size.width as f64;
    let height = size.height as f64;
    if width > 2.0 && height > 2.0 {
        Some((width, height))
    } else {
        None
    }
}

fn view_box(view: Option<&NSView>) -> (f64, f64, f64, f64) {
    let Some(view) = view else {
        return (0.0, 0.0, 0.0, 0.0);
    };
    let frame = view.frame();
    (
        frame.origin.x as f64,
        frame.origin.y as f64,
        frame.size.width as f64,
        frame.size.height as f64,
    )
}

fn filling_child(content: &NSView) -> Option<Retained<NSView>> {
    let children = content.subviews();
    let mut best: Option<Retained<NSView>> = None;
    let mut area = 0.0;
    for index in 0..children.count() {
        let child = children.objectAtIndex(index);
        let frame = child.frame();
        let next = frame.size.width as f64 * frame.size.height as f64;
        if next > area {
            area = next;
            best = Some(child);
        }
    }
    best
}

fn content_is_fitted(window: &NSWindow) -> bool {
    let Some(content) = window.contentView() else {
        return false;
    };
    let frame = window.frame().size;
    let (x, y, width, height) = view_box(Some(&content));
    !content_needs_fit(frame.width as f64, frame.height as f64, x, y, width, height)
}

fn content_needs_fit(
    window_w: f64,
    window_h: f64,
    content_x: f64,
    content_y: f64,
    content_w: f64,
    content_h: f64,
) -> bool {
    if window_w < 2.0 || window_h < 2.0 || content_w < 2.0 || content_h < 2.0 {
        return false;
    }
    content_x.abs() > 0.5
        || content_y.abs() > 0.5
        || (content_w - window_w).abs() > 0.5
        || (content_h - window_h).abs() > 0.5
}

fn child_needs_fit(
    parent_w: f64,
    parent_h: f64,
    child_x: f64,
    child_y: f64,
    child_w: f64,
    child_h: f64,
) -> bool {
    if parent_w < 2.0 || parent_h < 2.0 || child_w < parent_w * 0.5 || child_h < 2.0 {
        return false;
    }
    child_x.abs() > 0.5
        || child_y.abs() > 0.5
        || (child_w - parent_w).abs() > 0.5
        || (child_h - parent_h).abs() > 0.5
}

fn note_fit(kind: &str, target: &NSRect, before: NSRect) {
    let body = serde_json::json!({
        "kind": kind,
        "target_w": target.size.width as f64,
        "target_h": target.size.height as f64,
        "before_x": before.origin.x as f64,
        "before_y": before.origin.y as f64,
        "before_w": before.size.width as f64,
        "before_h": before.size.height as f64,
    });
    let _ = std::fs::create_dir_all("/tmp/superreader-spike");
    let _ = std::fs::write(
        format!("/tmp/superreader-spike/content-fit-{kind}.json"),
        body.to_string(),
    );
}

fn content_window_frame(view: &NSView) -> NSRect {
    let frame = view.frame();
    match unsafe { view.superview() } {
        Some(parent) => parent.convertRect_toView(frame, None),
        None => frame,
    }
}

/// Window frame, content rect, and the stage view, in points. The shell's
/// available height is whichever of these the layout actually fills.
pub fn write_bounds(stage: &NSView) {
    let Some(window) = stage.window() else {
        return;
    };
    let frame = window.frame();
    let layout = window.contentLayoutRect();
    let content = window.contentView().map(|view| content_window_frame(&view));
    let parent = unsafe { stage.superview() }.map(|view| view.bounds().size);
    let stage_bounds = stage.bounds().size;
    let screen = window.screen().map(|screen| {
        serde_json::json!({
            "frame_w": screen.frame().size.width as f64,
            "frame_h": screen.frame().size.height as f64,
            "visible_w": screen.visibleFrame().size.width as f64,
            "visible_h": screen.visibleFrame().size.height as f64,
            "scale": screen.backingScaleFactor() as f64,
        })
    });
    let body = serde_json::json!({
        "scale": window.backingScaleFactor() as f64,
        "fullscreen": window.styleMask().contains(NSWindowStyleMask::FullScreen),
        "window": rect_json(frame),
        "content_layout": rect_json(layout),
        "content": content.map(rect_json),
        "stage_w": stage_bounds.width as f64,
        "stage_h": stage_bounds.height as f64,
        "parent_w": parent.map(|size| size.width as f64),
        "parent_h": parent.map(|size| size.height as f64),
        "screen": screen,
    });
    let _ = std::fs::create_dir_all("/tmp/superreader-spike");
    let _ = std::fs::write("/tmp/superreader-spike/bounds.json", body.to_string());
}

fn rect_json(rect: NSRect) -> serde_json::Value {
    serde_json::json!({
        "x": rect.origin.x as f64,
        "y": rect.origin.y as f64,
        "w": rect.size.width as f64,
        "h": rect.size.height as f64,
    })
}

fn button_window_frame(button: &NSButton) -> NSRect {
    let view: &NSView = unsafe { &*std::ptr::from_ref(button).cast::<NSView>() };
    let frame = view.frame();
    match unsafe { view.superview() } {
        Some(parent) => parent.convertRect_toView(frame, None),
        None => frame,
    }
}

fn edge_box(rect: NSRect) -> (f64, f64, f64, f64) {
    (
        rect.origin.x,
        rect.origin.x + rect.size.width,
        rect.origin.y,
        rect.origin.y + rect.size.height,
    )
}

pub fn measure_traffic_bar(window: &NSWindow) -> Option<TrafficLights> {
    let close = window.standardWindowButton(NSWindowButton::CloseButton)?;
    let mini = window.standardWindowButton(NSWindowButton::MiniaturizeButton)?;
    let zoom = window.standardWindowButton(NSWindowButton::ZoomButton)?;
    let (close_min_x, close_max_x, close_min_y, close_max_y) =
        edge_box(button_window_frame(&close));
    let (mini_min_x, mini_max_x, mini_min_y, mini_max_y) = edge_box(button_window_frame(&mini));
    let (zoom_min_x, zoom_max_x, zoom_min_y, zoom_max_y) = edge_box(button_window_frame(&zoom));
    if !close_min_x.is_finite() || !zoom_max_x.is_finite() {
        return None;
    }
    if close_min_x < 0.0 || zoom_max_x <= close_min_x {
        return None;
    }
    let width = centered_bar_width(close_min_x, zoom_max_x);
    if !(8.0..=400.0).contains(&width) {
        return None;
    }
    Some(TrafficLights {
        close_min_x,
        close_max_x,
        close_min_y,
        close_max_y,
        mini_min_x,
        mini_max_x,
        mini_min_y,
        mini_max_y,
        zoom_min_x,
        zoom_max_x,
        zoom_min_y,
        zoom_max_y,
        width,
    })
}

fn toolbar_style(window: &NSWindow) -> isize {
    let Some(toolbar) = window.toolbar() else {
        return -1;
    };
    let sel = nsh_platform_macos::objc2::runtime::Sel::register(c"toolbarStyle");
    let responds: bool = unsafe { msg_send![&*toolbar, respondsToSelector: sel] };
    if !responds {
        return -1;
    }
    unsafe { msg_send![&*toolbar, toolbarStyle] }
}

fn note_bar_context(window: &NSWindow, rt: &Runtime) {
    let style = toolbar_style(window);
    let mask = window.styleMask().bits();
    let unseen = rt.toolbar_style.get() == -2;
    if !unseen && (rt.toolbar_style.get() != style || rt.style_mask.get() != mask) {
        rt.mark_bar_dirty();
    }
    rt.toolbar_style.set(style);
    rt.style_mask.set(mask);
}

fn settle_bar(window: &NSWindow, rt: &Runtime) {
    note_bar_context(window, rt);
    if !rt.bar_dirty.get() {
        return;
    }
    if window.inLiveResize() {
        rt.bar_seen.set(0.0);
        rt.bar_stable.set(0);
        rt.frame_key.set((0, 0));
        return;
    }
    let Some(measured) = measure_traffic_bar(window) else {
        return;
    };
    let frame = window.frame().size;
    let frame_key = (frame.width.round() as i32, frame.height.round() as i32);
    let width_ok = rt.bar_seen.get() > 1.0 && (rt.bar_seen.get() - measured.width).abs() <= 0.5;
    let frame_ok = rt.frame_key.get() == frame_key && frame_key != (0, 0);
    if width_ok && frame_ok {
        let count = rt.bar_stable.get().saturating_add(1);
        rt.bar_stable.set(count);
        if count >= 1 {
            rt.bar_dirty.set(false);
            rt.bar_gen.set(rt.bar_gen.get().saturating_add(1));
        }
    } else {
        rt.bar_stable.set(0);
    }
    rt.bar_seen.set(measured.width);
    rt.frame_key.set(frame_key);
    rt.bar_w.set(measured.width);
    *rt.traffic.borrow_mut() = Some(measured);
}

fn note_blur(window: &NSWindow) {
    if !crate::verify_session::VerifyHooks::from_env().should_resign() {
        return;
    }
    unsafe {
        let _: () = msg_send![window, resignKeyWindow];
    }
}

fn remember_shared(rt: &Runtime, alpha: f64) {
    if shell_on() && rt.reading.get() && !flag("SPIKE_LIGHTS") {
        rt.note_chrome_status(alpha, true);
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn button_alpha(window: &NSWindow) -> Option<f64> {
    let button = window.standardWindowButton(NSWindowButton::CloseButton)?;
    let alpha: f64 = unsafe { msg_send![&*button, alphaValue] };
    alpha.is_finite().then_some(alpha)
}

fn title_field_alpha(rt: &Runtime) -> Option<f64> {
    let field = rt.hover_title.borrow().clone()?;
    let alpha: f64 = unsafe { msg_send![&*field, alphaValue] };
    alpha.is_finite().then_some(alpha)
}

pub fn append_chrome_status(rt: &Runtime) {
    let line = serde_json::json!({
        "t_ms": unix_ms(),
        "reading": rt.reading.get(),
        "title_opacity": rt.title_opacity.get(),
        "chrome_shared": rt.chrome_shared.get(),
        "lights": rt.lights.get(),
    });
    let _ = crate::verify_session::VerifyHooks::from_env().append_status(&line.to_string());
}

fn file_flag(path: impl AsRef<std::path::Path>) -> Option<bool> {
    let raw = std::fs::read_to_string(path).ok()?;
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    Some(text == "1")
}

fn note_immersive(rt: &Runtime) {
    let Some(want) = file_flag("/tmp/superreader-spike/bar-immersive") else {
        return;
    };
    if rt.immersive.get() != want {
        rt.immersive.set(want);
        rt.mark_bar_dirty();
    }
}

fn note_fullscreen_hook(window: &NSWindow, rt: &Runtime) {
    let Some(raw) = crate::verify_session::VerifyHooks::from_env().read_text("bar-fullscreen")
    else {
        return;
    };
    let text = raw.trim();
    if text.is_empty() {
        return;
    }
    let want = text == "1";
    let is_full = window.styleMask().contains(NSWindowStyleMask::FullScreen);
    if want == is_full {
        rt.bar_fullscreen_sent.set(false);
        return;
    }
    if rt.bar_fullscreen_sent.get() {
        return;
    }
    window.toggleFullScreen(None);
    rt.bar_fullscreen_sent.set(true);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FullscreenDecision {
    enter: bool,
    exit: bool,
}

#[derive(Clone, Copy, Debug)]
struct FullscreenInput {
    is_full: bool,
    want: bool,
    exit_file: bool,
    enter_sent: bool,
    exit_sent: bool,
    window_ready: bool,
    /// `fullscreen-arm` is `0` while the driver still needs the windowed frame.
    hold_arm: bool,
    /// The content view already filled the window on the previous turn.
    /// A view fitted in this same turn is not ready: the shelf has not
    /// published that height yet, and fullscreen would snapshot the short one.
    content_fitted: bool,
    /// The reader settle loop has stopped calling `setFrame`. Entering before
    /// that lets the next frame cancel `toggleFullScreen:`, and `enter_sent`
    /// then never tries again.
    frame_settled: bool,
}

fn fullscreen_decision(input: FullscreenInput) -> FullscreenDecision {
    let none = FullscreenDecision {
        enter: false,
        exit: false,
    };
    if !input.want {
        return none;
    }
    if input.exit_file {
        if input.is_full && !input.exit_sent {
            return FullscreenDecision {
                enter: false,
                exit: true,
            };
        }
        return none;
    }
    if input.hold_arm || !input.content_fitted {
        return none;
    }
    if input.is_full || !input.window_ready || input.enter_sent || !input.frame_settled {
        return none;
    }
    FullscreenDecision {
        enter: true,
        exit: false,
    }
}

/// A pinned window size must not be reapplied while fullscreen is in progress.
/// The exit file is what returns the window to that size.
fn hold_pinned_frame(is_full: bool, enter_sent: bool, exit_sent: bool) -> bool {
    if is_full {
        return false;
    }
    if enter_sent && !exit_sent {
        return false;
    }
    true
}

fn request_fullscreen(
    window: &NSWindow,
    rt: &Runtime,
    is_full: bool,
    armed: bool,
    content_fitted: bool,
) {
    let decision = fullscreen_decision(FullscreenInput {
        is_full,
        want: flag("SPIKE_FULLSCREEN"),
        exit_file: std::path::Path::new("/tmp/superreader-spike/fullscreen-exit").is_file(),
        enter_sent: rt.fullscreen_sent.get(),
        exit_sent: rt.fullscreen_exit_sent.get(),
        window_ready: armed && window.isVisible() && window.isKeyWindow(),
        hold_arm: file_flag("/tmp/superreader-spike/fullscreen-arm") == Some(false),
        content_fitted,
        frame_settled: rt.settled.get(),
    });
    if decision.enter {
        window.toggleFullScreen(None);
        rt.fullscreen_sent.set(true);
    } else if decision.exit {
        window.toggleFullScreen(None);
        rt.fullscreen_exit_sent.set(true);
    }
}

fn shelf_event_window(event: &NSEvent, mtm: MainThreadMarker) -> Option<Retained<NSWindow>> {
    if let Some(window) = event.window(mtm) {
        return Some(window);
    }
    // A scroll posted with CGEvent reaches this monitor before AppKit has
    // attached a window. The cursor was warped onto the shelf, so the key
    // window is the one that should move.
    objc2_app_kit::NSApplication::sharedApplication(mtm).keyWindow()
}

fn shelf_event_point(event: &NSEvent, window: &NSWindow, mtm: MainThreadMarker) -> NSPoint {
    if event.window(mtm).is_some() {
        return event.locationInWindow();
    }
    let mouse = NSEvent::mouseLocation();
    let frame = window.frame();
    NSPoint::new(mouse.x - frame.origin.x, mouse.y - frame.origin.y)
}

fn install_shelf_wheel() {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    let block = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        let e = unsafe { event.as_ref() };
        let phase = e.phase().0 as u64;
        let momentum = e.momentumPhase().0 as u64;
        let precise = e.hasPreciseScrollingDeltas();
        let scroll_x = e.scrollingDeltaX();
        let scroll_y = e.scrollingDeltaY();
        let line_x = e.deltaX();
        let line_y = e.deltaY();
        let scrolls_shelf = crate::visit::wheel_scrolls_shelf(shell_on());
        let page_took = if scrolls_shelf {
            false
        } else {
            crate::web::take_epub_wheel(
                phase,
                momentum,
                crate::visit::wheel_points(precise, scroll_x, line_x),
                crate::visit::wheel_points(precise, scroll_y, line_y),
            )
        };
        match crate::visit::wheel_owner(scrolls_shelf, page_took) {
            crate::visit::WheelOwner::Page => return std::ptr::null_mut(),
            crate::visit::WheelOwner::Through => return event.as_ptr(),
            crate::visit::WheelOwner::Shelf => {}
        }
        let Some(mtm) = MainThreadMarker::new() else {
            return event.as_ptr();
        };
        let Some(window) = shelf_event_window(e, mtm) else {
            return event.as_ptr();
        };
        let Some(content) = window.contentView() else {
            return event.as_ptr();
        };
        let height = content.frame().size.height;
        if !height.is_finite() || height <= 1.0 {
            return event.as_ptr();
        }
        let point = shelf_event_point(e, &window, mtm);
        let x = point.x as f32;
        let y = (height - point.y) as f32;
        if !crate::form::shelf_hit_contains(x, y) {
            return event.as_ptr();
        }
        let mut dy = e.scrollingDeltaY();
        if dy.abs() < 0.01 {
            dy = e.deltaY() * 16.0;
        }
        if dy.abs() < 0.01 {
            return event.as_ptr();
        }
        crate::form::nudge_shelf(-dy as f32);
        std::ptr::null_mut()
    });
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::ScrollWheel, &block)
    };
    std::mem::forget(monitor);
    let keys = block2::RcBlock::new(move |event: std::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        crate::web::abandon_swipe();
        event.as_ptr()
    });
    let keys = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &keys)
    };
    std::mem::forget(keys);
}

fn expose_window(window: &NSWindow) {
    let title = NSString::from_str("Slowpage");
    let _: () = unsafe { msg_send![window, setAccessibilityTitle: &*title] };
}

fn pinned_frame() -> Option<(f64, f64)> {
    let raw = crate::verify_session::VerifyHooks::from_env().read_text("shell-frame")?;
    let mut parts = raw.split_whitespace();
    let width: f64 = parts.next()?.parse().ok()?;
    let height: f64 = parts.next()?.parse().ok()?;
    if width > 50.0 && height > 50.0 {
        Some((width, height))
    } else {
        None
    }
}

fn apply_frame(window: &NSWindow, screen: &NSScreen, frame: &Frame, animate: bool, place: bool) {
    let visible = screen.visibleFrame();
    let width = frame.window_w;
    let height = frame.window_h;
    let current = window.frame();
    let size_ok =
        (current.size.width - width).abs() < 2.0 && (current.size.height - height).abs() < 2.0;
    if size_ok && !place {
        return;
    }
    let x = if place {
        visible.origin.x + (visible.size.width - width) / 2.0
    } else {
        current.origin.x + (current.size.width - width) / 2.0
    };
    let y = if place {
        visible.origin.y + (visible.size.height - height) / 2.0
    } else {
        current.origin.y + current.size.height - height
    };
    window.setFrame_display_animate(
        NSRect::new(
            NSPoint::new(x.max(visible.origin.x), y.max(visible.origin.y)),
            NSSize::new(width, height),
        ),
        true,
        animate,
    );
}

fn reading_chrome_alpha(window: &NSWindow, rt: &Runtime) -> f64 {
    let now = Instant::now();
    let dt = rt
        .chrome_tick
        .get()
        .map(|then| {
            let ms = now.saturating_duration_since(then).as_millis();
            u32::try_from(ms).unwrap_or(u32::MAX)
        })
        .unwrap_or(0);
    rt.chrome_tick.set(Some(now));
    let sample = crate::reading::ChromeSample {
        generation: rt.chrome_gen.get(),
        pointer_at_top: near_top(window),
        focused: window.isKeyWindow(),
        dt_ms: dt,
        cut: rt.chrome_cut.replace(false),
    };
    let clock = crate::reading::step_chrome(rt.chrome_fade.get(), sample);
    rt.chrome_fade.set(clock);
    f64::from(clock.alpha)
}

fn hover_target(window: &NSWindow) -> f64 {
    if near_top(window) {
        1.0
    } else {
        0.0
    }
}

fn near_top(window: &NSWindow) -> bool {
    let mouse = objc2_app_kit::NSEvent::mouseLocation();
    let frame = window.frame();
    let top = frame.origin.y + frame.size.height;
    mouse.x >= frame.origin.x
        && mouse.x <= frame.origin.x + frame.size.width
        && mouse.y <= top
        && mouse.y >= top - 56.0
}

fn approach(current: f64, target: f64) -> f64 {
    let delta = (target - current).clamp(-0.25, 0.25);
    (current + delta).clamp(0.0, 1.0)
}

fn paint_lights(window: &NSWindow, alpha: f64) {
    for button in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        let Some(button) = window.standardWindowButton(button) else {
            continue;
        };
        // Fade with alpha while the lights are showing. Hide them only once
        // they are gone, so a capture does not keep the colored discs, and
        // hide them after the contents item has already taken its slot.
        unsafe {
            if alpha < 0.02 {
                let _: () = msg_send![&*button, setAlphaValue: 0.0_f64];
                let _: () = msg_send![&*button, setHidden: true];
                let _: () = msg_send![&*button, setEnabled: false];
            } else {
                let _: () = msg_send![&*button, setHidden: false];
                let _: () = msg_send![&*button, setAlphaValue: alpha];
                let _: () = msg_send![&*button, setEnabled: true];
            }
        }
    }
}

fn place_title(window: &NSWindow, rt: &Runtime, alpha: f64) {
    let shown = rt.title.borrow().clone();
    // The installed app keeps the product name on the window. The dev spike
    // still puts the book title there.
    if !shown.is_empty() && !rt.product.get() {
        window.setTitle(&NSString::from_str(&shown));
    }
    // The toolbar field is created only after the contents icon exists. The
    // hover label does not wait for that, and a hidden title bar does not
    // draw the toolbar item anyway.
    place_hover_title(window, rt, &shown, alpha);
    let Some(field) = rt.field.borrow().clone() else {
        return;
    };
    let face = NSString::from_str(title_face(&shown));
    if let Some(font) = NSFont::fontWithName_size(&face, 13.0) {
        unsafe {
            let _: () = msg_send![&*field, setFont: &*font];
        }
    }
    unsafe {
        let value = NSString::from_str(&shown);
        let _: () = msg_send![&*field, setStringValue: &*value];
        let _: () = msg_send![&*field, setAlignment: objc2_app_kit::NSTextAlignment(1)];
        let _: () = msg_send![&*field, setAlphaValue: alpha];
        let _: () = msg_send![&*field, setHidden: alpha < 0.02];
    }
}

/// The contents control in window coordinates. Its leading edge is 26pt past
/// the zoom button, and it shares that button's vertical center.
pub(crate) fn contents_button_frame(
    zoom_x: f64,
    zoom_y: f64,
    zoom_w: f64,
    zoom_h: f64,
    icon: f64,
) -> (f64, f64, f64, f64) {
    let x = zoom_x + zoom_w + 26.0;
    let y = zoom_y + (zoom_h - icon) / 2.0;
    (x, y, icon, icon)
}

fn place_contents_button(window: &NSWindow, rt: &Runtime, alpha: f64) {
    let Some(content) = window.contentView() else {
        return;
    };
    let Some(zoom) = window.standardWindowButton(NSWindowButton::ZoomButton) else {
        return;
    };
    let Some(image) = icons::contents_image() else {
        return;
    };
    image.setTemplate(false);
    let mtm = MainThreadMarker::from(&*window);
    let existing = rt.contents_button.borrow().clone();
    let button = if let Some(existing) = existing {
        existing
    } else {
        let created = unsafe { NSButton::buttonWithImage_target_action(&image, None, None, mtm) };
        created.setBordered(false);
        created.setImageScaling(NSImageScaling::ScaleNone);
        let label = NSString::from_str("Contents");
        unsafe {
            let _: () = msg_send![&*created, setAccessibilityLabel: &*label];
        }
        let view: Retained<NSView> = created.clone().into_super().into_super();
        view.setIdentifier(Some(&NSString::from_str(CONTENTS_ID)));
        *rt.contents_button.borrow_mut() = Some(created.clone());
        created
    };
    button.setImage(Some(&image));
    let zoom_frame = button_window_frame(&zoom);
    let (x, y, width, height) = contents_button_frame(
        zoom_frame.origin.x,
        zoom_frame.origin.y,
        zoom_frame.size.width,
        zoom_frame.size.height,
        image.size().width.max(18.0),
    );
    let host = content_window_frame(&content);
    let local_x = x - host.origin.x;
    let local_bottom = y - host.origin.y;
    let flipped: bool = unsafe { msg_send![&*content, isFlipped] };
    let local_y = if flipped {
        content.bounds().size.height - local_bottom - height
    } else {
        local_bottom
    };
    let _ = mtm;
    button.setFrame(NSRect::new(
        NSPoint::new(local_x, local_y),
        NSSize::new(width, height),
    ));
    button.setAlphaValue(alpha);
    button.setHidden(alpha < 0.02);
    button.setEnabled(alpha >= 0.9);
    content.addSubview(&button);
}

pub fn bind_contents(rt: &Runtime, target: &AnyObject) {
    let Some(button) = rt.contents_button.borrow().clone() else {
        return;
    };
    unsafe {
        let _: () = msg_send![&*button, setTarget: target];
        let _: () = msg_send![&*button, setAction: sel!(showContents:)];
    }
}

/// The book title's frame in the content view. Width leaves the traffic lights
/// clear. The 18-point row sits against the top. A flipped content view counts
/// y from that top; an ordinary one counts y from the bottom.
pub(crate) fn hover_title_frame(
    content_w: f64,
    content_h: f64,
    flipped: bool,
) -> (f64, f64, f64, f64) {
    let width = (content_w - 180.0).max(40.0);
    let x = (content_w - width) / 2.0;
    let y = if flipped { 18.0 } else { content_h - 36.0 };
    (x, y, width, 18.0)
}

fn place_hover_title(window: &NSWindow, rt: &Runtime, shown: &str, alpha: f64) {
    let Some(content) = window.contentView() else {
        return;
    };
    let mtm = MainThreadMarker::from(&*window);
    let existing = rt.hover_title.borrow().clone();
    let field = if let Some(existing) = existing {
        existing
    } else {
        let created = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        created.setTextColor(Some(&NSColor::colorWithRed_green_blue_alpha(
            0.36, 0.33, 0.29, 1.0,
        )));
        created.setBezeled(false);
        created.setBordered(false);
        created.setDrawsBackground(false);
        created.setEditable(false);
        created.setSelectable(false);
        *rt.hover_title.borrow_mut() = Some(created.clone());
        created
    };
    // The page view is added after the first sync. Re-adding keeps the label
    // above that view. A hidden title bar still does not draw the toolbar item.
    content.addSubview(&field);
    let face = NSString::from_str(title_face(shown));
    if let Some(font) = NSFont::fontWithName_size(&face, 13.0) {
        unsafe {
            let _: () = msg_send![&*field, setFont: &*font];
        }
    }
    let flipped: bool = unsafe { msg_send![&*content, isFlipped] };
    let (x, y, width, height) = hover_title_frame(
        content.bounds().size.width,
        content.bounds().size.height,
        flipped,
    );
    unsafe {
        let value = NSString::from_str(shown);
        let _: () = msg_send![&*field, setStringValue: &*value];
        let _: () = msg_send![&*field, setAlignment: objc2_app_kit::NSTextAlignment(1)];
        let _: () = msg_send![&*field, setAlphaValue: alpha];
        let _: () = msg_send![&*field, setHidden: alpha < 0.02];
        let _: () = msg_send![
            &*field,
            setFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
        ];
    }
}

fn apply_appearance(window: &NSWindow) {
    // AppOptions pins the window to aqua. This overrides that for the icon shots.
    let Ok(mode) = std::env::var("SPIKE_APPEARANCE") else {
        return;
    };
    let name: &NSString = if mode == "dark" {
        unsafe { NSAppearanceNameDarkAqua }
    } else {
        unsafe { NSAppearanceNameAqua }
    };
    let Some(appearance) = NSAppearance::appearanceNamed(name) else {
        return;
    };
    window.setAppearance(Some(&appearance));
}

const TITLE_ID: &str = "books.title";
const CONTENTS_ID: &str = "books.contents";
const TITLE_W: f64 = 120.0;

define_class!(
    #[unsafe(super(NSTextField))]
    #[thread_kind = MainThreadOnly]
    #[ivars = f64]
    struct TitleField;

    unsafe impl NSObjectProtocol for TitleField {}

    impl TitleField {
        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size(&self) -> NSSize {
            let full: NSSize = unsafe { msg_send![super(self), intrinsicContentSize] };
            let width = if full.width.is_finite() {
                full.width.min(TITLE_W)
            } else {
                TITLE_W
            };
            NSSize::new(width, *self.ivars())
        }
    }
);

fn title_label(mtm: MainThreadMarker, row: f64) -> Retained<NSTextField> {
    let height = row.max(18.0);
    let field = mtm.alloc::<TitleField>().set_ivars(height);
    let field: Retained<TitleField> = unsafe {
        msg_send![
            super(field),
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(TITLE_W, height))
        ]
    };
    let field = field.into_super();
    field.setBezeled(false);
    field.setBordered(false);
    field.setDrawsBackground(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setMaximumNumberOfLines(1);
    field.setLineBreakMode(NSLineBreakMode::ByTruncatingMiddle);
    // The toolbar measures this label from its text. A PDF title is wider
    // than the row, and that width hides the label and the right-hand icons
    // behind the overflow chevron.
    field.setPreferredMaxLayoutWidth(TITLE_W);
    field
}
pub const TOOLBAR_ORDER: [&str; 5] = [
    "books.contents",
    "books.bookmarks",
    "books.search",
    "books.add-bookmark",
    "books.notes",
];

fn place_contents(window: &NSWindow, rt: &Runtime, alpha: f64) {
    if std::env::var("SPIKE_ICON").ok().as_deref() == Some("0") {
        if let Some(button) = rt.contents_button.borrow().clone() {
            button.setHidden(true);
        }
        if let Some(toolbar) = window.toolbar() {
            clear_contents(&toolbar);
        }
        return;
    }
    // The hidden title bar does not draw the toolbar item, so the lights stay
    // and the contents control does not. The same list glyph is placed on the
    // content view, 26pt past the zoom button.
    place_contents_button(window, rt, alpha);
    let Some(toolbar) = window.toolbar() else {
        return;
    };
    // Inserting the item while the lights are faded puts it in the
    // traffic-light row, and it stays there after they return. Wait until
    // the buttons are shown, which is the placement that leaves 26 pt.
    if rt.contents_bar.borrow().is_none() && alpha < 0.9 {
        return;
    }
    ensure_contents(window, rt, &toolbar);
    for item in toolbar.items() {
        let Some(view) = item.view() else {
            continue;
        };
        view.setHidden(alpha < 0.02);
        view.setAlphaValue(alpha);
    }
    if let Some(view) = item_view(&toolbar, CONTENTS_ID) {
        record_gap(window, rt, &view);
    }
}

/// The library title is the card header. A window title or a reader toolbar
/// item would sit in that band while the window is windowed and vanish in
/// fullscreen.
fn clear_shell_chrome(window: &NSWindow) {
    window.setTitle(&NSString::from_str(""));
    expose_window(window);
    if let Some(toolbar) = window.toolbar() {
        clear_contents(&toolbar);
    }
}

fn clear_contents(toolbar: &NSToolbar) {
    if item_view(toolbar, CONTENTS_ID).is_none() {
        return;
    }
    toolbar.setItemIdentifiers(&NSArray::from_slice(&[] as &[&NSString]));
}

fn ensure_contents(window: &NSWindow, rt: &Runtime, toolbar: &NSToolbar) {
    if rt.contents_bar.borrow().is_some() {
        return;
    }
    let contents = if std::env::var("SPIKE_ICON").ok().as_deref() == Some("sf") {
        reference_symbol()
    } else {
        icons::contents_image()
    };
    let Some(contents) = contents else {
        return;
    };
    let Some(bookmark) = icons::bookmark_image() else {
        return;
    };
    let Some(search) = icons::search_image() else {
        return;
    };
    let Some(add) = icons::add_bookmark_image() else {
        return;
    };
    let Some(notes) = icons::notes_image() else {
        return;
    };
    let mtm = MainThreadMarker::from(&*window);
    let row = row_height(window);
    let field = title_label(mtm, row);
    field.setTextColor(Some(&NSColor::colorWithRed_green_blue_alpha(
        0.36, 0.33, 0.29, 1.0,
    )));
    *rt.field.borrow_mut() = Some(field.clone());
    let bar = mtm.alloc::<ContentsBar>().set_ivars(ContentsIvars {
        contents,
        bookmark,
        search,
        add,
        notes,
        title: field,
        row,
    });
    let bar: Retained<ContentsBar> = unsafe { msg_send![super(bar), init] };
    toolbar.setAutosavesConfiguration(false);
    toolbar.setDisplayMode(NSToolbarDisplayMode::IconOnly);
    toolbar.setDelegate(Some(ProtocolObject::from_ref(&*bar)));
    toolbar.setItemIdentifiers(&bar_ids());
    *rt.contents_bar.borrow_mut() = Some(bar);
}

fn item_view(toolbar: &NSToolbar, identifier: &str) -> Option<Retained<NSView>> {
    toolbar.items().iter().find_map(|item| {
        if item.itemIdentifier().to_string() == identifier {
            item.view()
        } else {
            None
        }
    })
}

fn bar_ids() -> Retained<NSArray<NSString>> {
    let contents = NSString::from_str(CONTENTS_ID);
    let bookmarks = NSString::from_str("books.bookmarks");
    let search = NSString::from_str("books.search");
    let title = NSString::from_str(TITLE_ID);
    let add = NSString::from_str("books.add-bookmark");
    let notes = NSString::from_str("books.notes");
    let space = unsafe { &*NSToolbarFlexibleSpaceItemIdentifier };
    NSArray::from_slice(&[
        &*contents,
        &*bookmarks,
        &*search,
        space,
        &*title,
        space,
        &*add,
        &*notes,
    ])
}

fn row_height(window: &NSWindow) -> f64 {
    let Some(button) = window.standardWindowButton(NSWindowButton::CloseButton) else {
        return 18.0;
    };
    let host: Option<Retained<NSView>> = unsafe { msg_send![&*button, superview] };
    let Some(host) = host else {
        return 18.0;
    };
    let height = host.frame().size.height;
    if height.is_finite() && height > 18.0 {
        height
    } else {
        18.0
    }
}

fn reference_symbol() -> Option<Retained<NSImage>> {
    let base = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str("list.bullet"),
        None,
    )?;
    let weight = unsafe { NSFontWeightRegular };
    let config = NSImageSymbolConfiguration::configurationWithPointSize_weight(16.0, weight);
    let image = base.imageWithSymbolConfiguration(&config).unwrap_or(base);
    image.setTemplate(true);
    Some(image)
}

fn record_gap(window: &NSWindow, rt: &Runtime, view: &NSView) {
    let Some(zoom) = window.standardWindowButton(NSWindowButton::ZoomButton) else {
        return;
    };
    let zoom_bounds: NSRect = unsafe { msg_send![&*zoom, bounds] };
    let zoom_win: NSRect =
        unsafe { msg_send![&*zoom, convertRect: zoom_bounds, toView: std::ptr::null::<NSView>()] };
    let item_win = view.convertRect_toView(view.bounds(), None);
    let gap = item_win.origin.x - (zoom_win.origin.x + zoom_win.size.width);
    let zoom_mid = zoom_win.origin.y + zoom_win.size.height / 2.0;
    let item_mid = item_win.origin.y + item_win.size.height / 2.0;
    let dy = item_mid - zoom_mid;
    let first = rt.icon_gap.get().is_nan();
    rt.icon_gap.set(gap);
    rt.icon_dy.set(dy);
    if first {
        eprintln!("contents toolbar gap {gap:.1} pt, vertical delta {dy:.1} pt");
    }
}

struct ContentsIvars {
    contents: Retained<NSImage>,
    bookmark: Retained<NSImage>,
    search: Retained<NSImage>,
    add: Retained<NSImage>,
    notes: Retained<NSImage>,
    title: Retained<NSTextField>,
    row: f64,
}

define_class!(
    #[unsafe(super(objc2::runtime::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ContentsIvars]
    struct ContentsBar;

    unsafe impl NSObjectProtocol for ContentsBar {}

    unsafe impl NSToolbarDelegate for ContentsBar {}

    impl ContentsBar {
        #[unsafe(method_id(toolbar:itemForItemIdentifier:willBeInsertedIntoToolbar:))]
        fn item_for(
            &self,
            _toolbar: &NSToolbar,
            identifier: &NSString,
            _insert: bool,
        ) -> Option<Retained<NSToolbarItem>> {
            let mtm = MainThreadMarker::from(self);
            let id = identifier.to_string();
            if id == TITLE_ID {
                let item =
                    NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), identifier);
                item.setBordered(false);
                let field = self.ivars().title.clone();
                let hosted = field.into_super().into_super();
                item.setView(Some(&hosted));
                Some(item)
            } else {
                let image = match id.as_str() {
                    CONTENTS_ID => Some(&self.ivars().contents),
                    "books.bookmarks" => Some(&self.ivars().bookmark),
                    "books.search" => Some(&self.ivars().search),
                    "books.add-bookmark" => Some(&self.ivars().add),
                    "books.notes" => Some(&self.ivars().notes),
                    _ => None,
                };
                image.map(|image| icon_item(identifier, image, self.ivars().row, mtm))
            }
        }

        #[unsafe(method_id(toolbarDefaultItemIdentifiers:))]
        fn default_ids(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSString>> {
            bar_ids()
        }

        #[unsafe(method_id(toolbarAllowedItemIdentifiers:))]
        fn allowed_ids(&self, _toolbar: &NSToolbar) -> Retained<NSArray<NSString>> {
            bar_ids()
        }
    }
);

fn icon_item(
    identifier: &NSString,
    image: &NSImage,
    row: f64,
    mtm: MainThreadMarker,
) -> Retained<NSToolbarItem> {
    let item = NSToolbarItem::initWithItemIdentifier(NSToolbarItem::alloc(mtm), identifier);
    item.setBordered(false);
    // A template image tints white in this toolbar, so draw the black raster.
    image.setTemplate(false);
    let view = NSImageView::imageViewWithImage(image, mtm);
    view.setImageAlignment(NSImageAlignment::AlignCenter);
    view.setImageScaling(NSImageScaling::ScaleNone);
    let height = row.max(18.0);
    view.setFrame(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(image.size().width.max(18.0), height),
    ));
    let hosted = view.into_super().into_super();
    item.setView(Some(&hosted));
    item
}

fn observe_lights(window: &NSWindow, stage_h: f64) -> (bool, f64) {
    let frame = window.frame().size;
    let top = window
        .contentView()
        .map(|view| {
            let content = content_window_frame(&view);
            crate::tokens::content_top(frame.height, content.size.height, content.origin.y, stage_h)
        })
        .unwrap_or(0.0);
    let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .map(|kind| light_box(window, kind, frame.height));
    let shown = crate::tokens::lights_shown(&buttons, frame.width as f32, frame.height as f32);
    (shown, top)
}

fn light_box(window: &NSWindow, kind: NSWindowButton, window_h: f64) -> crate::tokens::LightBox {
    let Some(button) = window.standardWindowButton(kind) else {
        return crate::tokens::LightBox {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            shown: false,
        };
    };
    let hidden: bool = unsafe { msg_send![&*button, isHidden] };
    let alpha: f64 = unsafe { msg_send![&*button, alphaValue] };
    let rect = button_window_frame(&button);
    let y = window_h - (rect.origin.y + rect.size.height);
    crate::tokens::LightBox {
        x: rect.origin.x as f32,
        y: y as f32,
        w: rect.size.width as f32,
        h: rect.size.height as f32,
        shown: !hidden && crate::tokens::light_alpha_shown(alpha),
    }
}

fn write_json(window: &NSWindow, rt: &Runtime, fullscreen: bool) {
    let (visible, top) = observe_lights(window, rt.stage_h.get());
    rt.lights_visible.set(visible);
    rt.content_top.set(top);
    let page = rt.page.get();
    let frame = window.frame();
    let button = button_alpha(window)
        .map(serde_json::Value::from)
        .unwrap_or(serde_json::Value::Null);
    let field_alpha = title_field_alpha(rt)
        .map(serde_json::Value::from)
        .unwrap_or(serde_json::Value::Null);
    let lights = rt.traffic.borrow().unwrap_or(TrafficLights {
        close_min_x: 0.0,
        close_max_x: 0.0,
        close_min_y: 0.0,
        close_max_y: 0.0,
        mini_min_x: 0.0,
        mini_max_x: 0.0,
        mini_min_y: 0.0,
        mini_max_y: 0.0,
        zoom_min_x: 0.0,
        zoom_max_x: 0.0,
        zoom_min_y: 0.0,
        zoom_max_y: 0.0,
        width: 0.0,
    });
    let content = view_box(window.contentView().as_deref());
    let mut body = serde_json::json!({
        "window_w": frame.size.width,
        "window_h": frame.size.height,
        "content_x": content.0,
        "content_y": content.1,
        "content_w": content.2,
        "content_h": content.3,
        "page_x": page.x,
        "page_y": page.y,
        "page_w": page.w,
        "page_h": page.h,
        "stage_w": page.stage_w,
        "stage_h": page.stage_h,
        "top": TOP,
        "side": SIDE,
        "fullscreen": fullscreen,
        "lights": rt.lights.get(),
        "t_ms": unix_ms(),
        "reading": rt.reading.get(),
        "title_opacity": rt.title_opacity.get(),
        "chrome_shared": rt.chrome_shared.get(),
        "button_alpha": button,
        "title_field_present": rt.hover_title.borrow().is_some(),
        "title_field_alpha": field_alpha,
        "focused": window.isKeyWindow(),
        "pointer_at_top": near_top(window),
        "generation": rt.chrome_gen.get(),
        "lights_visible": rt.lights_visible.get(),
        "content_top": rt.content_top.get(),
        "ratio_w": page.ratio_w,
        "ratio_h": page.ratio_h,
        "title": rt.title.borrow().clone(),
        "appearance": window.effectiveAppearance().name().to_string(),
        "items": TOOLBAR_ORDER,
        "icon_gap": finite_json(rt.icon_gap.get()),
        "icon_dy": finite_json(rt.icon_dy.get()),
        "icon_frame": icon_frame(window),
        "close": button_frame(window, NSWindowButton::CloseButton),
        "miniaturize": button_frame(window, NSWindowButton::MiniaturizeButton),
        "zoom": button_frame(window, NSWindowButton::ZoomButton),
        "pid": std::process::id(),
    });
    let bar = serde_json::json!({
        "bar_x": 0.0,
        "bar_w": rt.bar_w.get(),
        "bar_h": page.stage_h,
        "bar_dirty": rt.bar_dirty.get(),
        "bar_gen": rt.bar_gen.get(),
        "immersive": rt.immersive.get(),
        "light_w": lights.width,
        "close_min_x": lights.close_min_x,
        "close_max_x": lights.close_max_x,
        "close_min_y": lights.close_min_y,
        "close_max_y": lights.close_max_y,
        "mini_min_x": lights.mini_min_x,
        "mini_max_x": lights.mini_max_x,
        "mini_min_y": lights.mini_min_y,
        "mini_max_y": lights.mini_max_y,
        "zoom_min_x": lights.zoom_min_x,
        "zoom_max_x": lights.zoom_max_x,
        "zoom_min_y": lights.zoom_min_y,
        "zoom_max_y": lights.zoom_max_y,
    });
    if let (Some(map), Some(extra)) = (body.as_object_mut(), bar.as_object()) {
        for (key, value) in extra {
            map.insert(key.clone(), value.clone());
        }
    }
    let text = serde_json::to_string_pretty(&body).unwrap_or_default();
    let dir = "/tmp/superreader-spike";
    let _ = std::fs::create_dir_all(dir);
    let pid = std::process::id();
    for path in [
        format!("{dir}/chrome.json"),
        format!("{dir}/chrome-{pid}.json"),
    ] {
        let tmp = format!("{path}.{pid}.tmp");
        if std::fs::write(&tmp, &text).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

fn button_frame(window: &NSWindow, kind: NSWindowButton) -> serde_json::Value {
    let Some(button) = window.standardWindowButton(kind) else {
        return serde_json::Value::Null;
    };
    view_frame(&button)
}

fn icon_frame(window: &NSWindow) -> serde_json::Value {
    if let Some(button) = rt_contents_button(window) {
        return view_frame(&button);
    }
    let Some(toolbar) = window.toolbar() else {
        return serde_json::Value::Null;
    };
    let Some(view) = item_view(&toolbar, CONTENTS_ID) else {
        return serde_json::Value::Null;
    };
    view_frame(&view)
}

fn rt_contents_button(window: &NSWindow) -> Option<Retained<NSView>> {
    let content = window.contentView()?;
    for sub in content.subviews().iter() {
        let ident = sub.identifier();
        if ident.as_deref().map(NSString::to_string).as_deref() == Some(CONTENTS_ID) {
            return Some(sub.clone());
        }
    }
    None
}

fn view_frame(view: &NSView) -> serde_json::Value {
    let bounds = view.bounds();
    let rect: NSRect =
        unsafe { msg_send![view, convertRect: bounds, toView: std::ptr::null::<NSView>()] };
    serde_json::json!({
        "x": rect.origin.x,
        "y": rect.origin.y,
        "w": rect.size.width,
        "h": rect.size.height,
    })
}

fn flag(name: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some("1")
}

fn finite_json(value: f64) -> serde_json::Value {
    if value.is_finite() {
        serde_json::json!(value)
    } else {
        serde_json::Value::Null
    }
}

fn spike_window() -> Option<(f64, f64)> {
    parse_spike_window(&std::env::var("SPIKE_WINDOW").ok()?)
}

fn parse_spike_window(raw: &str) -> Option<(f64, f64)> {
    let (width, height) = raw.split_once('x')?;
    let width: f64 = width.trim().parse().ok()?;
    let height: f64 = height.trim().parse().ok()?;
    if !(200.0..=4000.0).contains(&width) || !(200.0..=4000.0).contains(&height) {
        return None;
    }
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() < 0.02, "{got} vs {want}");
    }

    #[test]
    fn full_content_fits_the_window_and_ignores_a_matched_frame() {
        assert!(!content_needs_fit(1440.0, 855.0, 0.0, 0.0, 1440.0, 855.0));
        assert!(!content_needs_fit(578.0, 528.0, 0.0, 0.0, 578.2, 528.0));
        assert!(content_needs_fit(1512.0, 949.0, 0.0, 0.0, 1440.0, 855.0));
        assert!(!content_needs_fit(1512.0, 949.0, 0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn a_short_root_is_fitted_and_a_traffic_button_is_not() {
        assert!(child_needs_fit(1512.0, 949.0, 0.0, 0.0, 1440.0, 855.0));
        assert!(!child_needs_fit(1512.0, 949.0, 0.0, 0.0, 1512.0, 949.0));
        assert!(!child_needs_fit(1512.0, 949.0, 12.0, 900.0, 14.0, 14.0));
    }

    #[test]
    fn fullscreen_waits_until_the_window_is_up_and_does_not_toggle_twice() {
        let idle = FullscreenInput {
            is_full: false,
            want: true,
            exit_file: false,
            enter_sent: false,
            exit_sent: false,
            window_ready: false,
            hold_arm: false,
            content_fitted: true,
            frame_settled: true,
        };
        assert_eq!(
            fullscreen_decision(idle),
            FullscreenDecision {
                enter: false,
                exit: false,
            }
        );
        let ready = FullscreenInput {
            window_ready: true,
            ..idle
        };
        assert!(fullscreen_decision(ready).enter);
        let unsettled = FullscreenInput {
            frame_settled: false,
            ..ready
        };
        assert!(!fullscreen_decision(unsettled).enter);
        assert!(!fullscreen_decision(unsettled).exit);
        let short = FullscreenInput {
            content_fitted: false,
            ..ready
        };
        assert!(!fullscreen_decision(short).enter);
        assert!(!fullscreen_decision(short).exit);
        let held = FullscreenInput {
            hold_arm: true,
            ..ready
        };
        assert!(!fullscreen_decision(held).enter);
        assert!(!fullscreen_decision(held).exit);
        let sent = FullscreenInput {
            enter_sent: true,
            ..ready
        };
        assert_eq!(
            fullscreen_decision(sent),
            FullscreenDecision {
                enter: false,
                exit: false,
            }
        );
        let inside = FullscreenInput {
            is_full: true,
            exit_file: true,
            enter_sent: true,
            ..ready
        };
        assert!(fullscreen_decision(inside).exit);
        assert!(!hold_pinned_frame(false, true, false));
        assert!(hold_pinned_frame(false, true, true));
        assert!(!hold_pinned_frame(true, true, false));
    }

    #[test]
    fn spike_window_parses_a_frame_and_rejects_a_partial_one() {
        assert_eq!(parse_spike_window("640x960"), Some((640.0, 960.0)));
        assert_eq!(parse_spike_window(" 700 x 800 "), Some((700.0, 800.0)));
        assert_eq!(parse_spike_window("640"), None);
        assert_eq!(parse_spike_window("10x10"), None);
    }

    #[test]
    fn paperback_on_a_1000_by_1200_frame_is_the_page_plus_the_top_mat() {
        let frame = fit(129.0, 198.0, 1000.0, 1200.0);
        close(frame.page_h, 1056.0 - TOP);
        close(frame.page_w, frame.page_h * 129.0 / 198.0);
        close(frame.window_h, frame.page_h + TOP);
        close(frame.window_w, frame.page_w + SIDE * 2.0);
        assert!(frame.window_w <= 1000.0 * FIT + 0.1);
        assert!(frame.window_h <= 1200.0 * FIT + 0.1);
    }

    #[test]
    fn the_page_starts_below_the_traffic_lights() {
        let (_, _, _, top) = content_box(800.0, 1000.0);
        assert!(top >= 44.0);
    }

    #[test]
    fn the_contents_control_sits_26pt_past_the_zoom_button() {
        let (x, y, width, height) = contents_button_frame(40.0, 800.0, 14.0, 16.0, 18.0);
        close(x, 80.0);
        close(y, 799.0);
        close(width, 18.0);
        close(height, 18.0);
        let zoom_mid = 800.0 + 8.0;
        let icon_mid = y + height / 2.0;
        close(icon_mid, zoom_mid);
    }

    #[test]
    fn the_hover_title_is_centered_and_clears_the_lights() {
        let (x, y, width, height) = hover_title_frame(582.0, 781.0, false);
        close(width, 402.0);
        close(x, 90.0);
        close(y, 745.0);
        close(height, 18.0);
        let (fx, fy, _, _) = hover_title_frame(582.0, 781.0, true);
        close(fx, x);
        close(fy, 18.0);
        assert!(x >= 70.0, "the title must clear the traffic lights");
    }

    #[test]
    fn the_reader_page_is_not_pushed_aside_for_the_traffic_lights() {
        let (_, _, left, _) = content_box(582.0, 781.0);
        close(reader_page_x(left, 0.0), SIDE);
        let pushed = content_page_x(582.0, 478.0, 82.0).unwrap();
        assert!(pushed > reader_page_x(left, 0.0) + 20.0);
    }

    #[test]
    fn the_page_is_centered_in_the_area_beside_the_bar() {
        let x = content_page_x(1100.0, 400.0, 98.0).unwrap();
        let page_mid = x + 200.0;
        let area_mid = 98.0 + (1100.0 - 98.0) / 2.0;
        close(page_mid, area_mid);
        assert!(content_page_x(1100.0, 400.0, 0.0).is_none());
        assert!(content_page_x(200.0, 180.0, 98.0).is_none());
    }

    #[test]
    fn the_high_button_joins_the_other_two_centers() {
        // Window y grows upward. The zoom button in the reader shot sits
        // about 8pt closer to the top, so its center is 8pt greater.
        let deltas = traffic_center_deltas(&[806.0, 806.0, 814.0]);
        assert!(deltas[0].abs() <= 0.5);
        assert!(deltas[1].abs() <= 0.5);
        close(deltas[2], -8.0);
        let agreed = traffic_center_deltas(&[806.0, 806.0, 806.2]);
        assert!(agreed.iter().all(|delta| delta.abs() <= 0.5));
        assert!(
            traffic_center_deltas(&[806.0]).is_empty()
                || traffic_center_deltas(&[806.0]) == vec![0.0]
        );
    }

    #[test]
    fn the_bar_width_centers_the_traffic_lights() {
        let close_min_x = 12.5;
        let zoom_max_x = 70.0;
        let width = centered_bar_width(close_min_x, zoom_max_x);
        close(width, 82.5);
        close(close_min_x, width - zoom_max_x);
    }

    #[test]
    fn a_cjk_title_uses_songti() {
        assert_eq!(title_face("朝花夕拾"), "Songti SC");
        assert_eq!(title_face("Pride and Prejudice"), "Iowan Old Style");
    }
}
