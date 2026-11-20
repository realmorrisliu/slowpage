use std::sync::Mutex;

use crate::form::{immersive_frame, reading_frame, Rect};
use crate::tokens::{CARD_RADIUS, CHAT_PAD_X, INSET, TOGGLE_SIZE, TOGGLE_TOP};

pub const EASE_MS: u32 = crate::reading::SLIDE_MS as u32;

const LANDED: f64 = 1.0 - 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trip {
    leg: u64,
    origin: f64,
    goal: f64,
    span_ms: u32,
    elapsed_ms: u32,
    fit_immersive: bool,
}

impl Trip {
    fn at(immersive: bool) -> Self {
        let end = if immersive { 1.0 } else { 0.0 };
        Self {
            leg: 0,
            origin: end,
            goal: end,
            span_ms: 0,
            elapsed_ms: 0,
            fit_immersive: immersive,
        }
    }

    pub fn leg(self) -> u64 {
        self.leg
    }

    pub fn moving(self) -> bool {
        self.span_ms > 0 && self.elapsed_ms < self.span_ms
    }

    pub fn u(self) -> f64 {
        if self.span_ms == 0 {
            return self.goal;
        }
        let t = (f64::from(self.elapsed_ms) / f64::from(self.span_ms)).clamp(0.0, 1.0);
        let eased = (1.0 - (std::f64::consts::PI * t).cos()) / 2.0;
        self.origin + (self.goal - self.origin) * eased
    }

    fn aim(mut self, immersive: bool) -> Self {
        let goal = if immersive { 1.0 } else { 0.0 };
        if (self.goal - goal).abs() < 1e-9 {
            return self;
        }
        let from = self.u();
        let distance = (goal - from).abs();
        if distance < 1e-6 {
            self.origin = goal;
            self.goal = goal;
            self.span_ms = 0;
            self.elapsed_ms = 0;
            self.fit_immersive = immersive;
            return self;
        }
        self.leg = self.leg.wrapping_add(1);
        self.origin = from;
        self.goal = goal;
        self.span_ms = ((distance * f64::from(EASE_MS)).round() as u32).max(1);
        self.elapsed_ms = 0;
        self
    }

    fn clock(mut self, elapsed_ms: u32) -> Self {
        if self.span_ms == 0 {
            return self;
        }
        if elapsed_ms >= self.span_ms {
            let immersive = self.goal >= 0.5;
            self.origin = self.goal;
            self.span_ms = 0;
            self.elapsed_ms = 0;
            self.fit_immersive = immersive;
            return self;
        }
        self.elapsed_ms = elapsed_ms;
        self
    }
}

static TRIP: Mutex<Trip> = Mutex::new(Trip {
    leg: 0,
    origin: 0.0,
    goal: 0.0,
    span_ms: 0,
    elapsed_ms: 0,
    fit_immersive: false,
});

fn lock() -> std::sync::MutexGuard<'static, Trip> {
    TRIP.lock().unwrap_or_else(|poison| poison.into_inner())
}

pub fn current() -> Trip {
    *lock()
}

pub fn settle(immersive: bool) {
    let mut trip = lock();
    let leg = trip.leg.wrapping_add(1);
    *trip = Trip {
        leg,
        ..Trip::at(immersive)
    };
}

pub fn retarget(immersive: bool) -> Trip {
    let mut trip = lock();
    *trip = trip.aim(immersive);
    *trip
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

pub fn present(width: f64, height: f64, top: f64, margin_shown: bool, trip: Trip) -> Presented {
    let start = reading_frame(width, height, top, 0.0, margin_shown);
    let end = immersive_frame(width, height);
    let u = trip.u().clamp(0.0, 1.0);
    let card = mix_rect(start.card, end.card, u);
    let margin = margin_shown.then(|| {
        let from = start.margin.unwrap_or(Rect {
            x: width,
            y: 0.0,
            w: 0.0,
            h: height.max(1.0),
        });
        mix_rect(
            from,
            Rect {
                x: width,
                y: 0.0,
                w: 0.0,
                h: height.max(1.0),
            },
            u,
        )
    });
    let margin = margin.filter(|rect| rect.w > 0.5);
    Presented {
        card,
        radius: CARD_RADIUS * (1.0 - u),
        shadow: 1.0 - u,
        margin,
        covers: start.covers && u < LANDED,
        page: if trip.fit_immersive {
            end.card
        } else {
            start.card
        },
        plate: u >= LANDED,
        panel_toggle: u < LANDED,
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

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            settle(false);
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
        settle(false);
        let wide = present(1440.0, 900.0, 0.0, true, current());
        near(wide.card, 6.0, 6.0, 1188.0, 888.0);
        near(wide.page, 6.0, 6.0, 1188.0, 888.0);
        assert!((wide.radius - 12.0).abs() < 0.01);
        assert!((wide.shadow - 1.0).abs() < 0.01);
        near(wide.margin.unwrap(), 1200.0, 0.0, 240.0, 900.0);
        assert!(!wide.covers && !wide.plate && wide.panel_toggle);
        assert!(pointer_turns(current()));

        let enter = retarget(true);
        assert!(enter.moving());
        assert!(!pointer_turns(enter));
        let at_start = present(1440.0, 900.0, 0.0, true, advance(0));
        near(at_start.page, 6.0, 6.0, 1188.0, 888.0);
        let mid = present(1440.0, 900.0, 0.0, true, advance(80));
        near(mid.card, 3.0, 3.0, 1314.0, 894.0);
        near(mid.page, 6.0, 6.0, 1188.0, 888.0);
        assert!((mid.radius - 6.0).abs() < 0.01);
        assert!((mid.shadow - 0.5).abs() < 0.01);
        near(mid.margin.unwrap(), 1320.0, 0.0, 120.0, 900.0);
        assert!(!mid.plate && mid.panel_toggle);
        let landed = present(1440.0, 900.0, 0.0, true, advance(160));
        near(landed.card, 0.0, 0.0, 1440.0, 900.0);
        near(landed.page, 0.0, 0.0, 1440.0, 900.0);
        assert!(landed.margin.is_none());
        assert!((landed.radius).abs() < 0.01);
        assert!(landed.shadow.abs() < 0.01);
        assert!(landed.plate && !landed.panel_toggle);
        assert!(pointer_turns(current()));

        let leaving = retarget(false);
        assert_eq!(leaving.span_ms, EASE_MS);
        let leave_start = present(1440.0, 900.0, 0.0, true, advance(0));
        assert!(leave_start.plate);
        near(leave_start.page, 0.0, 0.0, 1440.0, 900.0);
        let leave_mid = present(1440.0, 900.0, 0.0, true, advance(80));
        near(leave_mid.card, 3.0, 3.0, 1314.0, 894.0);
        near(leave_mid.page, 0.0, 0.0, 1440.0, 900.0);
        assert!(!leave_mid.plate);
        let back = present(1440.0, 900.0, 0.0, true, advance(160));
        near(back.card, 6.0, 6.0, 1188.0, 888.0);
        near(back.page, 6.0, 6.0, 1188.0, 888.0);

        retarget(true);
        advance(80);
        let turned = retarget(false);
        assert_eq!(turned.span_ms, 80);
        let halfway_back = present(1440.0, 900.0, 0.0, true, advance(40));
        assert!((current().u() - 0.25).abs() < 0.01);
        near(halfway_back.page, 6.0, 6.0, 1188.0, 888.0);
        let leg = current().leg();
        let again = retarget(false);
        assert_eq!(again.leg(), leg);
        assert_eq!(again.span_ms, 80);
        advance(80);
        assert!(!current().moving());
        assert!(!escape_stays(false, current()));

        retarget(true);
        advance(80);
        assert!(escape_stays(true, current()));
        retarget(false);
        assert!(escape_stays(false, current()));
        let leaving = current().leg();
        assert_eq!(retarget(false).leg(), leaving);
        advance(80);
        assert!(!escape_stays(false, current()));

        retarget(true);
        advance(80);
        let narrow = present(700.0, 800.0, 0.0, true, current());
        near(narrow.card, 3.0, 3.0, 694.0, 794.0);
        near(narrow.page, 6.0, 6.0, 688.0, 788.0);
        assert!(narrow.covers);
        near(narrow.margin.unwrap(), 580.0, 0.0, 120.0, 800.0);
        let column = present(818.0, 800.0, 0.0, true, current());
        let covered = present(817.0, 800.0, 0.0, true, current());
        assert!(!column.covers && covered.covers);
        advance(160);
        let full = present(1512.0, 982.0, 0.0, true, current());
        near(full.card, 0.0, 0.0, 1512.0, 982.0);
        settle(false);
        let schematic = present(1512.0, 982.0, 0.0, true, current());
        near(schematic.card, 6.0, 6.0, 1260.0, 970.0);

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
        retarget(true);
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
        let _ = present(700.0, 800.0, 0.0, true, current());
        let _ = present(1512.0, 982.0, 0.0, true, current());
        assert_eq!(stored.resume().page, 4);
        retarget(false);
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
        settle(false);
        near(
            present(1440.0, 900.0, 0.0, true, current()).page,
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
        let mut finger = crate::visit::Swipe::default();
        assert_eq!(
            finger.offer(
                crate::visit::WheelTick {
                    finger: crate::visit::Finger::Down,
                    momentum: false,
                    dx: 80.0,
                    dy: 0.0,
                },
                5,
                75,
            ),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert!(crate::visit::stop_for_ease(&mut finger));
        assert_eq!(
            finger.offer(
                crate::visit::WheelTick {
                    finger: crate::visit::Finger::Up,
                    momentum: false,
                    dx: 0.0,
                    dy: 0.0,
                },
                5,
                75,
            ),
            crate::visit::WheelMove::Pass
        );
        assert_eq!(stored.resume().page, 5);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
