use std::sync::Mutex;

use crate::form::{immersive_frame, reading_frame, Rect};
use crate::tokens::{CARD_RADIUS, CHAT_PAD_X, INSET, TOGGLE_SIZE, TOGGLE_TOP};

pub const EASE_MS: u32 = crate::reading::SLIDE_MS as u32;

const LANDED: f64 = 1.0 - 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Mix {
    hidden: f64,
    shown: f64,
    immersive: f64,
}

impl Mix {
    fn page(margin: bool) -> Self {
        if margin {
            Self {
                hidden: 0.0,
                shown: 1.0,
                immersive: 0.0,
            }
        } else {
            Self {
                hidden: 1.0,
                shown: 0.0,
                immersive: 0.0,
            }
        }
    }

    fn full() -> Self {
        Self {
            hidden: 0.0,
            shown: 0.0,
            immersive: 1.0,
        }
    }

    fn lerp(self, to: Self, u: f64) -> Self {
        Self {
            hidden: self.hidden + (to.hidden - self.hidden) * u,
            shown: self.shown + (to.shown - self.shown) * u,
            immersive: self.immersive + (to.immersive - self.immersive) * u,
        }
    }

    fn close(self, other: Self) -> bool {
        (self.hidden - other.hidden).abs() < 1e-9
            && (self.shown - other.shown).abs() < 1e-9
            && (self.immersive - other.immersive).abs() < 1e-9
    }

    fn travel(self, to: Self) -> f64 {
        ((self.hidden - to.hidden).abs()
            + (self.shown - to.shown).abs()
            + (self.immersive - to.immersive).abs())
            / 2.0
    }
}

fn fit_of(mix: Mix) -> Option<bool> {
    if mix.immersive >= 0.5 {
        None
    } else {
        Some(mix.shown >= mix.hidden)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trip {
    leg: u64,
    origin: Mix,
    goal: Mix,
    span_ms: u32,
    elapsed_ms: u32,
    /// The page box. `None` is the immersive frame. It changes when a leg lands.
    fit: Option<bool>,
    /// The page rectangle captured for this trip. Resize does not replace it.
    page_hold: Option<Rect>,
}

impl Trip {
    fn at(immersive: bool, margin: bool) -> Self {
        let pose = if immersive {
            Mix::full()
        } else {
            Mix::page(margin)
        };
        Self {
            leg: 0,
            origin: pose,
            goal: pose,
            span_ms: 0,
            elapsed_ms: 0,
            fit: fit_of(pose),
            page_hold: None,
        }
    }

    pub fn leg(self) -> u64 {
        self.leg
    }

    pub fn moving(self) -> bool {
        self.span_ms > 0 && self.elapsed_ms < self.span_ms
    }

    fn eased(self) -> f64 {
        if self.span_ms == 0 {
            return 1.0;
        }
        let t = (f64::from(self.elapsed_ms) / f64::from(self.span_ms)).clamp(0.0, 1.0);
        (1.0 - (std::f64::consts::PI * t).cos()) / 2.0
    }

    fn pose(self) -> Mix {
        if self.span_ms == 0 {
            self.goal
        } else {
            self.origin.lerp(self.goal, self.eased())
        }
    }

    pub fn u(self) -> f64 {
        self.pose().immersive
    }

    fn aim(mut self, immersive: bool, margin: bool) -> Self {
        let desired = if immersive {
            Mix::full()
        } else {
            Mix::page(margin)
        };
        if self.goal.close(desired) {
            return self;
        }
        let from = self.pose();
        let distance = from.travel(desired);
        if distance < 1e-6 {
            self.origin = desired;
            self.goal = desired;
            self.span_ms = 0;
            self.elapsed_ms = 0;
            self.fit = fit_of(desired);
            self.page_hold = None;
            return self;
        }
        self.leg = self.leg.wrapping_add(1);
        self.origin = from;
        self.goal = desired;
        self.span_ms = ((distance * f64::from(EASE_MS)).round() as u32).max(1);
        self.elapsed_ms = 0;
        self
    }

    fn clock(mut self, elapsed_ms: u32) -> Self {
        if self.span_ms == 0 {
            return self;
        }
        if elapsed_ms >= self.span_ms {
            self.origin = self.goal;
            self.span_ms = 0;
            self.elapsed_ms = 0;
            self.fit = fit_of(self.goal);
            self.page_hold = None;
            return self;
        }
        self.elapsed_ms = elapsed_ms;
        self
    }
}

static TRIP: Mutex<Trip> = Mutex::new(Trip {
    leg: 0,
    origin: Mix {
        hidden: 1.0,
        shown: 0.0,
        immersive: 0.0,
    },
    goal: Mix {
        hidden: 1.0,
        shown: 0.0,
        immersive: 0.0,
    },
    span_ms: 0,
    elapsed_ms: 0,
    fit: Some(false),
    page_hold: None,
});

fn lock() -> std::sync::MutexGuard<'static, Trip> {
    TRIP.lock().unwrap_or_else(|poison| poison.into_inner())
}

pub fn current() -> Trip {
    *lock()
}

pub fn settle(immersive: bool, margin: bool) {
    let mut trip = lock();
    let leg = trip.leg.wrapping_add(1);
    *trip = Trip {
        leg,
        ..Trip::at(immersive, margin)
    };
}

pub fn retarget(immersive: bool, margin: bool) -> Trip {
    crate::shelf_pass::park_reading();
    let mut trip = lock();
    *trip = trip.aim(immersive, margin);
    *trip
}

pub fn follow_margin(shown: bool) {
    let trip = current();
    if trip.moving() || trip.u() > 1e-3 {
        return;
    }
    let pose = trip.pose();
    let already = if shown {
        pose.shown >= 1.0 - 1e-6
    } else {
        pose.hidden >= 1.0 - 1e-6
    };
    if already {
        return;
    }
    settle(false, shown);
}

pub fn advance(elapsed_ms: u32) -> Trip {
    let mut trip = lock();
    *trip = trip.clock(elapsed_ms);
    *trip
}

pub fn pointer_turns(trip: Trip) -> bool {
    !trip.moving()
}

pub fn escape_stays(form_immersive: bool, trip: Trip) -> bool {
    form_immersive || trip.u() > 1e-3
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Presented {
    pub card: Rect,
    pub radius: f64,
    pub shadow: f64,
    pub margin: Option<Rect>,
    pub covers: bool,
    pub page: Rect,
    pub plate: bool,
    pub panel_toggle: bool,
}

pub fn present(width: f64, height: f64, top: f64, trip: Trip) -> Presented {
    let side = reading_side(trip);
    let hidden = reading_frame(width, height, top, side, false);
    let shown = reading_frame(width, height, top, side, true);
    let end = immersive_frame(width, height);
    let pose = trip.pose();
    let card = mix3(hidden.card, shown.card, end.card, pose);
    let margin = shown.margin.map(|from| {
        mix_rect(
            Rect {
                x: width,
                y: 0.0,
                w: 0.0,
                h: height.max(1.0),
            },
            from,
            pose.shown.clamp(0.0, 1.0),
        )
    });
    let margin = margin.filter(|rect| rect.w > 0.5);
    let fitted = match trip.fit {
        None => end.card,
        Some(true) => shown.card,
        Some(false) => hidden.card,
    };
    let page = held_page(trip, fitted);
    let mut presented = Presented {
        card,
        radius: CARD_RADIUS * (1.0 - pose.immersive),
        shadow: 1.0 - pose.immersive,
        margin,
        covers: shown.covers && pose.shown > 1e-3 && pose.immersive < LANDED,
        page,
        plate: pose.immersive >= LANDED,
        panel_toggle: pose.immersive < LANDED,
    };
    if pose.immersive < 1e-3 {
        if let Some(card) = crate::shelf_pass::card_blend(width, height, top) {
            let frame = reading_frame(width, height, top, side, crate::form::margin_shown());
            presented.card = card;
            presented.page = frame.card;
            presented.margin = frame.margin.filter(|rect| rect.w > 0.5);
            presented.covers = frame.covers;
        }
    }
    presented
}

fn held_page(trip: Trip, fitted: Rect) -> Rect {
    if !trip.moving() {
        let live = current();
        if live.leg() == trip.leg() && live.page_hold.is_some() {
            lock().page_hold = None;
        }
        return fitted;
    }
    if let Some(held) = trip.page_hold {
        return held;
    }
    let live = current();
    if live.leg() == trip.leg() {
        if let Some(held) = live.page_hold {
            return held;
        }
        lock().page_hold = Some(fitted);
    }
    fitted
}

fn mix3(hidden: Rect, shown: Rect, immersive: Rect, pose: Mix) -> Rect {
    Rect {
        x: hidden.x * pose.hidden + shown.x * pose.shown + immersive.x * pose.immersive,
        y: hidden.y * pose.hidden + shown.y * pose.shown + immersive.y * pose.immersive,
        w: hidden.w * pose.hidden + shown.w * pose.shown + immersive.w * pose.immersive,
        h: hidden.h * pose.hidden + shown.h * pose.shown + immersive.h * pose.immersive,
    }
}

/// One 28 point slot to the left of the panel toggle. The toggle's right
/// inset is `CHAT_PAD_X`. The gap between the two controls is the card inset.
pub fn button_seat(width: f64, top: f64) -> Rect {
    let size = f64::from(TOGGLE_SIZE);
    let gap = INSET;
    let right = f64::from(CHAT_PAD_X);
    Rect {
        x: width - right - size - gap - size,
        y: top + f64::from(TOGGLE_TOP),
        w: size,
        h: size,
    }
}

/// A shown reading sidebar insets the page card. Once immersive reading has
/// landed, the card is the window, so the inset is gone.
fn reading_side(trip: Trip) -> f64 {
    if trip.pose().immersive >= LANDED || crate::shelf_pass::moving() {
        0.0
    } else {
        crate::form::reading_sidebar_width()
    }
}

fn mix(from: f64, to: f64, u: f64) -> f64 {
    from + (to - from) * u
}

fn mix_rect(from: Rect, to: Rect, u: f64) -> Rect {
    Rect {
        x: mix(from.x, to.x, u),
        y: mix(from.y, to.y, u),
        w: mix(from.w, to.w, u),
        h: mix(from.h, to.h, u),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visit::PlaceFile;

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            settle(false, true);
        }
    }

    fn near(rect: Rect, x: f64, y: f64, w: f64, h: f64) {
        assert!((rect.x - x).abs() < 0.05, "{rect:?}");
        assert!((rect.y - y).abs() < 0.05, "{rect:?}");
        assert!((rect.w - w).abs() < 0.05, "{rect:?}");
        assert!((rect.h - h).abs() < 0.05, "{rect:?}");
    }

    #[test]
    fn an_immersive_ease_refits_once_and_drops_the_flying_page() {
        let _reset = Reset;
        settle(false, true);
        let wide = present(1440.0, 900.0, 0.0, current());
        near(wide.card, 6.0, 6.0, 1188.0, 888.0);
        near(wide.page, 6.0, 6.0, 1188.0, 888.0);
        assert!((wide.radius - 12.0).abs() < 0.01);
        assert!((wide.shadow - 1.0).abs() < 0.01);
        near(wide.margin.unwrap(), 1200.0, 0.0, 240.0, 900.0);
        assert!(!wide.covers && !wide.plate && wide.panel_toggle);
        assert!(pointer_turns(current()));

        let enter = retarget(true, true);
        assert!(enter.moving());
        assert!(!pointer_turns(enter));
        let at_start = present(1440.0, 900.0, 0.0, advance(0));
        near(at_start.page, 6.0, 6.0, 1188.0, 888.0);
        let mid = present(1440.0, 900.0, 0.0, advance(80));
        near(mid.card, 3.0, 3.0, 1314.0, 894.0);
        near(mid.page, 6.0, 6.0, 1188.0, 888.0);
        assert!((mid.radius - 6.0).abs() < 0.01);
        assert!((mid.shadow - 0.5).abs() < 0.01);
        near(mid.margin.unwrap(), 1320.0, 0.0, 120.0, 900.0);
        assert!(!mid.plate && mid.panel_toggle);
        let landed = present(1440.0, 900.0, 0.0, advance(160));
        near(landed.card, 0.0, 0.0, 1440.0, 900.0);
        near(landed.page, 0.0, 0.0, 1440.0, 900.0);
        assert!(landed.margin.is_none());
        assert!((landed.radius).abs() < 0.01);
        assert!(landed.shadow.abs() < 0.01);
        assert!(landed.plate && !landed.panel_toggle);
        assert!(pointer_turns(current()));

        let leaving = retarget(false, true);
        assert_eq!(leaving.span_ms, EASE_MS);
        let leave_start = present(1440.0, 900.0, 0.0, advance(0));
        assert!(leave_start.plate);
        near(leave_start.page, 0.0, 0.0, 1440.0, 900.0);
        let leave_mid = present(1440.0, 900.0, 0.0, advance(80));
        near(leave_mid.card, 3.0, 3.0, 1314.0, 894.0);
        near(leave_mid.page, 0.0, 0.0, 1440.0, 900.0);
        assert!(!leave_mid.plate);
        let back = present(1440.0, 900.0, 0.0, advance(160));
        near(back.card, 6.0, 6.0, 1188.0, 888.0);
        near(back.page, 6.0, 6.0, 1188.0, 888.0);

        retarget(true, true);
        advance(80);
        let turned = retarget(false, true);
        assert_eq!(turned.span_ms, 80);
        let halfway_back = present(1440.0, 900.0, 0.0, advance(40));
        assert!((current().u() - 0.25).abs() < 0.01);
        near(halfway_back.page, 6.0, 6.0, 1188.0, 888.0);
        let leg = current().leg();
        let again = retarget(false, true);
        assert_eq!(again.leg(), leg);
        assert_eq!(again.span_ms, 80);
        advance(80);
        assert!(!current().moving());
        assert!(!escape_stays(false, current()));

        retarget(true, true);
        advance(80);
        assert!(escape_stays(true, current()));
        retarget(false, true);
        assert!(escape_stays(false, current()));
        let leaving = current().leg();
        assert_eq!(retarget(false, true).leg(), leaving);
        advance(80);
        assert!(!escape_stays(false, current()));

        retarget(true, true);
        advance(80);
        let narrow = present(700.0, 800.0, 0.0, current());
        near(narrow.card, 3.0, 3.0, 694.0, 794.0);
        near(narrow.page, 6.0, 6.0, 688.0, 788.0);
        assert!(narrow.covers);
        near(narrow.margin.unwrap(), 580.0, 0.0, 120.0, 800.0);
        let column = present(818.0, 800.0, 0.0, current());
        let covered = present(817.0, 800.0, 0.0, current());
        assert!(!column.covers && covered.covers);
        advance(160);
        let full = present(1512.0, 982.0, 0.0, current());
        near(full.card, 0.0, 0.0, 1512.0, 982.0);
        settle(false, true);
        let schematic = present(1512.0, 982.0, 0.0, current());
        near(schematic.card, 6.0, 6.0, 1260.0, 970.0);

        settle(false, false);
        near(
            present(1440.0, 900.0, 0.0, current()).card,
            6.0,
            6.0,
            1428.0,
            888.0,
        );
        retarget(true, false);
        advance(80);
        let hidden_mid = present(1440.0, 900.0, 0.0, current());
        near(hidden_mid.card, 3.0, 3.0, 1434.0, 894.0);
        near(hidden_mid.page, 6.0, 6.0, 1428.0, 888.0);
        let turned_hidden = retarget(false, true);
        assert_eq!(turned_hidden.span_ms, EASE_MS);
        let held = present(1440.0, 900.0, 0.0, current());
        near(held.card, 3.0, 3.0, 1434.0, 894.0);
        near(held.page, 6.0, 6.0, 1428.0, 888.0);
        assert!(held.margin.is_none());
        advance(80);
        let returning = present(1440.0, 900.0, 0.0, current());
        near(returning.page, 6.0, 6.0, 1428.0, 888.0);
        near(returning.card, 4.5, 4.5, 1311.0, 891.0);
        advance(160);
        let shown_again = present(1440.0, 900.0, 0.0, current());
        near(shown_again.card, 6.0, 6.0, 1188.0, 888.0);
        near(shown_again.page, 6.0, 6.0, 1188.0, 888.0);

        retarget(true, true);
        let origin = present(1440.0, 900.0, 0.0, advance(0));
        near(origin.page, 6.0, 6.0, 1188.0, 888.0);
        advance(80);
        let origin_key = crate::chrome::fitted_page(
            origin.page.x,
            origin.page.y,
            origin.page.w,
            origin.page.h,
            612.0,
            792.0,
        )
        .key();
        for (width, height) in [
            (700.0, 800.0),
            (817.0, 800.0),
            (818.0, 800.0),
            (1512.0, 982.0),
        ] {
            let frame = present(width, height, 0.0, current());
            assert_eq!(frame.page, origin.page);
            let key = crate::chrome::fitted_page(
                frame.page.x,
                frame.page.y,
                frame.page.w,
                frame.page.h,
                612.0,
                792.0,
            )
            .key();
            assert_eq!(key, origin_key);
            let mut reader = crate::visit::Reader::default();
            let plan = crate::visit::layout_frame(
                &mut reader,
                crate::visit::FrameTick::new(true, true, false, true, true, false),
            );
            assert_eq!(plan.draw, crate::visit::Draw::Rest);
        }
        retarget(false, true);
        assert_eq!(present(700.0, 800.0, 0.0, current()).page, origin.page);
        retarget(true, true);
        assert_eq!(present(1512.0, 982.0, 0.0, current()).page, origin.page);
        advance(160);
        let refit = present(1512.0, 982.0, 0.0, current());
        near(refit.page, 0.0, 0.0, 1512.0, 982.0);
        let refit_key = crate::chrome::fitted_page(
            refit.page.x,
            refit.page.y,
            refit.page.w,
            refit.page.h,
            612.0,
            792.0,
        )
        .key();
        assert_ne!(refit_key, origin_key);
        let mut reader = crate::visit::Reader::default();
        let plan = crate::visit::layout_frame(
            &mut reader,
            crate::visit::FrameTick::new(false, true, false, true, true, false),
        );
        assert!(!matches!(plan.draw, crate::visit::Draw::Rest));
        settle(false, true);

        let seat = button_seat(1440.0, 0.0);
        near(seat, 1362.0, 10.0, 28.0, 28.0);
        let glyph = crate::icons::glyph_source("maximize").expect("maximize");
        assert!(glyph.contains("M2.25 6 L2.25 3.25"));
        assert!(glyph.contains("currentColor"));
        assert!(!glyph.contains("#1c1c1e"));

        let dir = std::env::temp_dir().join(format!("superreader-ease-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = "/books/field.pdf";
        crate::settings::record_open(&file, book, 4, Some(75), 1).unwrap();
        let stored = crate::settings::SettingsBook { file: &file, book };
        let missing = dir.join("missing.pdf");
        let width = 400.0;
        let notch = crate::visit::WheelTick {
            finger: crate::visit::Finger::None,
            momentum: false,
            dx: 48.0,
            dy: 0.0,
        };
        let mut swipe = crate::visit::Swipe::default();
        retarget(true, true);
        assert_eq!(
            crate::visit::offer_while_reading(&mut swipe, notch, 4, 75, current()),
            crate::visit::WheelMove::Swallow
        );
        assert!(swipe.commit_token().is_none());
        advance(160);
        assert!(pointer_turns(current()));
        assert_eq!(
            crate::visit::offer_while_reading(&mut swipe, notch, 4, 75, current()),
            crate::visit::WheelMove::Commit(1)
        );
        let flying = swipe.commit_token().unwrap();
        let glide = match crate::visit::pdf_wheel(
            crate::visit::WheelMove::Commit(1),
            flying,
            1,
            4,
            75,
            width,
            0.0,
            false,
            false,
        ) {
            crate::visit::PdfWheel::Begin(glide) => glide,
            other => panic!("the page did not take the flying slot: {other:?}"),
        };
        assert_eq!(
            crate::visit::offer_while_reading(&mut swipe, notch, 4, 75, current()),
            crate::visit::WheelMove::Commit(1)
        );
        let queued = swipe.commit_token().unwrap();
        assert_ne!(queued, flying);
        let mut held = swipe.clone();
        assert!(held.take_commit_post(flying));
        assert!(held.take_commit_post(queued));
        assert_eq!(stored.resume().page, 4);
        let mut refused = swipe.clone();
        if crate::books::can_open(missing.to_str().unwrap()) {
            crate::visit::stop_for_ease(&mut refused);
        }
        assert!(refused.take_commit_post(flying));
        assert_eq!(stored.resume().page, 4);
        let _ = present(700.0, 800.0, 0.0, current());
        let _ = present(1512.0, 982.0, 0.0, current());
        assert_eq!(stored.resume().page, 4);
        retarget(false, true);
        assert!(crate::visit::stop_for_ease(&mut swipe));
        assert!(matches!(
            crate::visit::glide_at(&glide, 160, 1),
            crate::visit::GlideAt::Landed { token, .. } if token == flying
        ));
        assert!(!crate::visit::accept_landed(&mut swipe, flying));
        assert!(!crate::visit::accept_landed(&mut swipe, queued));
        assert_eq!(stored.resume().page, 4);
        let mut reader = crate::visit::Reader::default();
        reader.swipe = swipe;
        assert!(!crate::visit::retire_restore(&mut reader));
        settle(false, true);
        near(
            present(1440.0, 900.0, 0.0, current()).page,
            6.0,
            6.0,
            1188.0,
            888.0,
        );
        assert_eq!(stored.resume().page, 4);
        let mut next = crate::visit::Swipe::default();
        assert_eq!(
            crate::visit::offer_while_reading(&mut next, notch, 4, 75, current()),
            crate::visit::WheelMove::Commit(1)
        );
        let fresh = next.commit_token().unwrap();
        assert!(crate::visit::accept_landed(&mut next, fresh));
        crate::settings::record_progress(&file, book, 5, Some(75), 0, "").unwrap();
        assert_eq!(stored.resume().page, 5);
        settle(false, true);
        let mut finger = crate::visit::Swipe::default();
        let down = crate::visit::WheelTick {
            finger: crate::visit::Finger::Down,
            momentum: false,
            dx: 80.0,
            dy: 0.0,
        };
        let up = crate::visit::WheelTick {
            finger: crate::visit::Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        assert!(matches!(
            crate::visit::offer_while_reading(&mut finger, down, 5, 75, current()),
            crate::visit::WheelMove::Follow(_)
        ));
        assert!(crate::visit::stop_for_ease(&mut finger));
        retarget(true, true);
        assert_eq!(
            crate::visit::offer_while_reading(&mut finger, up, 5, 75, current()),
            crate::visit::WheelMove::Swallow
        );
        assert!(finger.commit_token().is_none());
        assert_eq!(
            crate::visit::offer_while_reading(&mut finger, down, 5, 75, current()),
            crate::visit::WheelMove::Swallow
        );
        assert_eq!(
            crate::visit::offer_while_reading(&mut finger, up, 5, 75, current()),
            crate::visit::WheelMove::Swallow
        );
        assert!(finger.commit_token().is_none());
        assert!(matches!(
            crate::visit::pdf_wheel(
                crate::visit::WheelMove::Swallow,
                0,
                1,
                5,
                75,
                400.0,
                0.0,
                false,
                false,
            ),
            crate::visit::PdfWheel::Pass
        ));
        assert_eq!(stored.resume().page, 5);
        advance(160);
        assert!(matches!(
            crate::visit::offer_while_reading(&mut finger, down, 5, 75, current()),
            crate::visit::WheelMove::Follow(_)
        ));
        assert_eq!(
            crate::visit::offer_while_reading(&mut finger, up, 5, 75, current()),
            crate::visit::WheelMove::Commit(1)
        );
        let landed = finger.commit_token().unwrap();
        assert!(crate::visit::accept_landed(&mut finger, landed));
        crate::settings::record_progress(&file, book, 6, Some(75), 0, "").unwrap();
        assert_eq!(stored.resume().page, 6);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
