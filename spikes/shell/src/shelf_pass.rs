//! One progress between the shelf and the open book.
//! Zero is Full. One is the page and its margin.
//! The cover is its own rect and keeps the shape it started with.

use std::cell::RefCell;

use crate::form::{self, Rect};
use crate::tokens::{self, CHAT_W, INSET, SIDEBAR_W};

const TRIP_MS: f64 = 160.0;
/// The shelf grid is gone once progress reaches this.
const GRID_GONE: f64 = 0.42;
/// The page text stays out until progress reaches this.
const BODY_AT: f64 = 0.55;
/// The cover has arrived and is still opaque here, then fades out by `BODY_AT`.
const COVER_HOLD: f64 = 0.50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seat {
    Shelf,
    Reading,
}

struct Pass {
    leg: u64,
    from: f64,
    to: f64,
    span_ms: u32,
    elapsed_ms: u32,
    generation: u64,
    seat: Seat,
    path: String,
    cover: Option<Rect>,
}

impl Default for Pass {
    fn default() -> Self {
        Self {
            leg: 0,
            from: 0.0,
            to: 0.0,
            span_ms: 0,
            elapsed_ms: 0,
            generation: 0,
            seat: Seat::Shelf,
            path: String::new(),
            cover: None,
        }
    }
}

thread_local! {
    static PASS: RefCell<Pass> = RefCell::new(Pass::default());
}

fn with_pass<T>(body: impl FnOnce(&mut Pass) -> T) -> T {
    PASS.with(|pass| body(&mut pass.borrow_mut()))
}

pub(crate) fn reset() {
    with_pass(|pass| *pass = Pass::default());
}

pub(crate) fn intend_reading(path: &str, cover: Option<Rect>) {
    with_pass(|pass| {
        let settled_here = pass.span_ms == 0
            && pass.seat == Seat::Reading
            && (pass.to - 1.0).abs() < 1e-3
            && pass.path == path;
        if settled_here {
            return;
        }
        let from = progress_of(pass);
        let already_open = pass.span_ms == 0 && pass.seat == Seat::Reading && from > 1.0 - 1e-3;
        pass.generation = pass.generation.wrapping_add(1);
        pass.path = path.to_string();
        if cover.is_some() {
            pass.cover = cover;
        }
        if already_open {
            pass.from = 1.0;
            pass.to = 1.0;
            pass.span_ms = 0;
            pass.elapsed_ms = 0;
            pass.seat = Seat::Reading;
            return;
        }
        aim(pass, from, 1.0);
    });
}

pub(crate) fn intend_shelf() {
    with_pass(|pass| {
        let from = progress_of(pass);
        if pass.span_ms == 0 && pass.seat == Seat::Shelf && from < 1e-3 {
            return;
        }
        pass.generation = pass.generation.wrapping_add(1);
        aim(pass, from, 0.0);
    });
}

/// Immersive reading starts from the open book, so the shelf trip is over.
pub(crate) fn park_reading() {
    with_pass(|pass| {
        if pass.generation == 0 && pass.span_ms == 0 {
            return;
        }
        if pass.span_ms == 0 && pass.seat == Seat::Reading && (pass.to - 1.0).abs() < 1e-3 {
            return;
        }
        pass.leg = pass.leg.wrapping_add(1);
        pass.from = 1.0;
        pass.to = 1.0;
        pass.span_ms = 0;
        pass.elapsed_ms = 0;
        pass.seat = Seat::Reading;
    });
}

pub(crate) fn advance(elapsed_ms: u32) {
    with_pass(|pass| {
        if pass.span_ms == 0 {
            return;
        }
        if elapsed_ms >= pass.span_ms {
            pass.from = pass.to;
            pass.span_ms = 0;
            pass.elapsed_ms = 0;
            pass.seat = if pass.to >= 0.5 {
                Seat::Reading
            } else {
                Seat::Shelf
            };
            return;
        }
        pass.elapsed_ms = elapsed_ms;
    });
}

pub(crate) fn leg() -> u64 {
    with_pass(|pass| pass.leg)
}

pub(crate) fn moving() -> bool {
    with_pass(|pass| pass.span_ms > 0)
}

pub(crate) fn progress() -> f64 {
    with_pass(|pass| progress_of(pass))
}

pub(crate) fn body_alpha() -> f64 {
    with_pass(|pass| {
        if pass.generation == 0 {
            return 1.0;
        }
        if pass.span_ms == 0 {
            return if pass.seat == Seat::Reading { 1.0 } else { 0.0 };
        }
        alphas(progress_of(pass)).body
    })
}

pub(crate) fn grid_alpha() -> f64 {
    with_pass(|pass| {
        if pass.span_ms == 0 {
            return 1.0;
        }
        alphas(progress_of(pass)).grid
    })
}

/// The shelf layer covers the page only while its grid or its cover is up.
pub(crate) fn shelf_veil() -> f64 {
    with_pass(|pass| {
        if pass.span_ms == 0 {
            return 1.0;
        }
        let fade = alphas(progress_of(pass));
        if fade.grid <= 0.001 && fade.cover <= 0.001 {
            0.0
        } else {
            1.0
        }
    })
}

pub(crate) fn show_shelf_layer() -> bool {
    with_pass(|pass| {
        if pass.span_ms == 0 {
            return false;
        }
        let fade = alphas(progress_of(pass));
        fade.grid > 0.001 || fade.cover > 0.001
    })
}

pub(crate) fn reading_mounted(surface_reading: bool) -> bool {
    with_pass(|pass| {
        if pass.generation == 0 {
            return surface_reading;
        }
        if pass.span_ms > 0 {
            return alphas(progress_of(pass)).body > 0.001;
        }
        surface_reading && pass.seat == Seat::Reading
    })
}

/// A status may move the margin only for the book that has landed.
pub(crate) fn accepts_margin(book: &str) -> bool {
    with_pass(|pass| {
        if pass.generation == 0 {
            return true;
        }
        if pass.span_ms > 0 || pass.seat != Seat::Reading {
            return false;
        }
        pass.path == book
    })
}

pub(crate) fn card_blend(width: f64, height: f64, top: f64) -> Option<Rect> {
    let (moving_now, p) = with_pass(|pass| (pass.span_ms > 0, progress_of(pass)));
    if !moving_now {
        return None;
    }
    Some(lerp_rect(
        shelf_card(width, height, top),
        reading_card(width, height, top),
        p,
    ))
}

pub(crate) struct Flight {
    pub rect: Rect,
    pub alpha: f64,
    pub path: String,
}

pub(crate) fn flight(window_w: f64, window_h: f64, card: Rect) -> Option<Flight> {
    let (origin, p, path) = with_pass(|pass| {
        if pass.span_ms == 0 {
            return None;
        }
        let origin = pass.cover?;
        Some((origin, progress_of(pass), pass.path.clone()))
    })?;
    let (rect, alpha) = fly_cover(origin, card, p, window_w, window_h)?;
    if alpha <= 0.001 {
        return None;
    }
    Some(Flight { rect, alpha, path })
}

pub(crate) fn shelf_card(width: f64, height: f64, top: f64) -> Rect {
    let top = top.max(0.0);
    let chat = if tokens::chat_fits(width as f32, SIDEBAR_W as f32) {
        CHAT_W
    } else {
        0.0
    };
    Rect {
        x: SIDEBAR_W + INSET,
        y: top + INSET,
        w: (width - SIDEBAR_W - chat - INSET * 2.0).max(1.0),
        h: (height - top - INSET * 2.0).max(1.0),
    }
}

pub(crate) fn reading_card(width: f64, height: f64, top: f64) -> Rect {
    form::reading_frame(width, height, top, 0.0, form::margin_shown()).card
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Fade {
    pub grid: f64,
    pub body: f64,
    pub cover: f64,
}

pub(crate) fn alphas(progress: f64) -> Fade {
    let p = progress.clamp(0.0, 1.0);
    let cover = if p <= COVER_HOLD {
        1.0
    } else if p >= BODY_AT {
        0.0
    } else {
        1.0 - (p - COVER_HOLD) / (BODY_AT - COVER_HOLD)
    };
    Fade {
        grid: (1.0 - p / GRID_GONE).clamp(0.0, 1.0),
        body: ((p - BODY_AT) / (1.0 - BODY_AT)).clamp(0.0, 1.0),
        cover,
    }
}

fn progress_of(pass: &Pass) -> f64 {
    if pass.span_ms == 0 {
        return pass.to;
    }
    let t = (f64::from(pass.elapsed_ms) / f64::from(pass.span_ms)).clamp(0.0, 1.0);
    let eased = 0.5 - 0.5 * (std::f64::consts::PI * t).cos();
    pass.from + (pass.to - pass.from) * eased
}

fn aim(pass: &mut Pass, from: f64, to: f64) {
    if (from - to).abs() < 1e-4 {
        pass.from = to;
        pass.to = to;
        pass.span_ms = 0;
        pass.elapsed_ms = 0;
        pass.seat = if to >= 0.5 {
            Seat::Reading
        } else {
            Seat::Shelf
        };
        return;
    }
    pass.leg = pass.leg.wrapping_add(1);
    pass.from = from;
    pass.to = to;
    let distance = (to - from).abs();
    pass.span_ms = ((distance * TRIP_MS).round() as u32).max(1);
    pass.elapsed_ms = 0;
}

fn lerp_rect(from: Rect, to: Rect, p: f64) -> Rect {
    let p = p.clamp(0.0, 1.0);
    Rect {
        x: from.x + (to.x - from.x) * p,
        y: from.y + (to.y - from.y) * p,
        w: from.w + (to.w - from.w) * p,
        h: from.h + (to.h - from.h) * p,
    }
}

pub(crate) fn fly_cover(
    origin: Rect,
    card: Rect,
    progress: f64,
    window_w: f64,
    window_h: f64,
) -> Option<(Rect, f64)> {
    if origin.w < 1.0 || origin.h < 1.0 || !origin.w.is_finite() || !origin.h.is_finite() {
        return None;
    }
    let aspect = origin.w / origin.h;
    if !aspect.is_finite() || aspect <= 0.0 {
        return None;
    }
    let p = progress.clamp(0.0, 1.0);
    let move_u = (p / COVER_HOLD).clamp(0.0, 1.0);
    let target_h = (card.h * 0.55).max(1.0);
    let mut h = origin.h + (target_h - origin.h) * move_u;
    let mut w = h * aspect;
    let max_w = (card.w * 0.72).max(1.0);
    if w > max_w {
        w = max_w;
        h = w / aspect;
    }
    let ox = origin.x + origin.w * 0.5;
    let oy = origin.y + origin.h * 0.5;
    let cx = card.x + card.w * 0.5;
    let cy = card.y + card.h * 0.5;
    let mx = ox + (cx - ox) * move_u;
    let my = oy + (cy - oy) * move_u;
    let rect = contain(
        Rect {
            x: mx - w * 0.5,
            y: my - h * 0.5,
            w,
            h,
        },
        aspect,
        window_w,
        window_h,
    )?;
    Some((rect, alphas(p).cover))
}

fn contain(rect: Rect, aspect: f64, window_w: f64, window_h: f64) -> Option<Rect> {
    if window_w < 1.0 || window_h < 1.0 || !rect.w.is_finite() || !rect.h.is_finite() {
        return None;
    }
    let mut w = rect.w.max(1.0);
    let mut h = rect.h.max(1.0);
    if w > window_w {
        w = window_w;
        h = w / aspect;
    }
    if h > window_h {
        h = window_h;
        w = h * aspect;
    }
    let mut x = rect.x;
    let mut y = rect.y;
    if x < 0.0 {
        x = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if x + w > window_w {
        x = (window_w - w).max(0.0);
    }
    if y + h > window_h {
        y = (window_h - h).max(0.0);
    }
    Some(Rect { x, y, w, h })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Clean;

    impl Drop for Clean {
        fn drop(&mut self) {
            reset();
            crate::ease::settle(false, false);
        }
    }

    fn origin() -> Rect {
        Rect {
            x: 280.0,
            y: 220.0,
            w: 128.0,
            h: 192.0,
        }
    }

    fn live(book: &str) -> spike_library::margin::LivePage {
        spike_library::margin::LivePage {
            book: book.to_string(),
            spine: String::new(),
            page: 2,
            page_w: 400.0,
            page_h: 600.0,
            moving: false,
        }
    }

    #[test]
    fn shelf_enter_and_leave_follow_the_cosine() {
        let _clean = Clean;
        reset();
        intend_reading("a.pdf", Some(origin()));
        advance(80);
        let mid = progress();
        assert!((mid - 0.5).abs() < 0.02, "{mid}");
        let fade = alphas(mid);
        assert!(fade.grid <= 0.001);
        assert!(fade.body <= 0.001);
        assert!((fade.cover - 1.0).abs() < 0.001);
        advance(160);
        assert!(!moving());
        assert!((progress() - 1.0).abs() < 0.001);
        assert!(accepts_margin("a.pdf"));
        intend_shelf();
        advance(160);
        assert!(!moving());
        assert!(progress() < 0.001);
        assert!(!accepts_margin("a.pdf"));

        reset();
        intend_reading("a.pdf", Some(origin()));
        let mut first = None;
        let mut last = None;
        for ms in 0..=160 {
            advance(ms);
            let fade = alphas(progress());
            if fade.grid <= 0.001 && fade.body <= 0.001 {
                first.get_or_insert(ms);
                last = Some(ms);
            }
        }
        let span = last.unwrap() - first.unwrap();
        assert!((10..=16).contains(&span), "{first:?} {last:?}");
    }

    #[test]
    fn a_reverse_continues_from_the_current_progress() {
        let _clean = Clean;
        reset();
        intend_reading("a.pdf", Some(origin()));
        advance(80);
        let mid = progress();
        intend_shelf();
        assert!((progress() - mid).abs() < 0.02);
        assert!(moving());
        advance(40);
        assert!(progress() < mid - 0.15);
        advance(80);
        assert!(progress() < 0.02);
        assert!(!moving());
        assert!(!accepts_margin("a.pdf"));
    }

    #[test]
    fn the_last_book_lands_and_a_missed_book_keeps_its_draft() {
        let _clean = Clean;
        reset();
        let draft = spike_library::margin::CardDraft {
            key: spike_library::margin::CardKey {
                book: "a.pdf".into(),
                note_id: 3,
                kind: spike_library::margin::CardKind::Edit,
            },
            body: "改过".into(),
            composing: false,
            notice: String::new(),
        };
        let ink = spike_library::margin::MarginInk {
            drafts: vec![draft],
            ..spike_library::margin::MarginInk::default()
        };
        let empty = serde_json::json!({});
        intend_reading("a.pdf", None);
        advance(40);
        let held = crate::form::absorb_margin(&ink, &empty, &live("a.pdf"));
        assert!(held.place_book.is_empty());
        assert_eq!(held.drafts[0].body, "改过");
        intend_reading("b.pdf", None);
        advance(30);
        let still = crate::form::absorb_margin(&held, &empty, &live("a.pdf"));
        assert!(still.place_book.is_empty());
        advance(1_000);
        let late = crate::form::absorb_margin(&still, &empty, &live("a.pdf"));
        assert!(late.place_book.is_empty(), "{}", late.place_book);
        let landed = crate::form::absorb_margin(&still, &empty, &live("b.pdf"));
        assert_eq!(landed.place_book, "b.pdf");
        assert_eq!(landed.drafts[0].key.book, "a.pdf");
        assert_eq!(landed.drafts[0].body, "改过");
        intend_shelf();
        advance(1_000);
        let parked = crate::form::absorb_margin(&landed, &empty, &live("b.pdf"));
        assert_eq!(parked.place_book, "b.pdf");
        assert_eq!(parked.drafts.len(), 1);
    }

    #[test]
    fn the_cover_keeps_its_aspect_inside_each_window() {
        let _clean = Clean;
        let aspect = origin().w / origin().h;
        for (width, height) in [(578.0, 528.0), (1440.0, 900.0), (2560.0, 1440.0)] {
            let shelf = shelf_card(width, height, 0.0);
            let reading = form::reading_frame(width, height, 0.0, 0.0, true).card;
            assert!(shelf.x >= 0.0 && shelf.y >= 0.0);
            assert!(shelf.x + shelf.w <= width + 0.1);
            assert!(shelf.y + shelf.h <= height + 0.1);
            assert!(reading.w > 1.0 && reading.h > 1.0);
            assert!(reading.x + reading.w <= width + 0.1);
            for step in [0.0, 0.25, 0.5, 0.8, 1.0] {
                let card = lerp_rect(shelf, reading, step);
                assert!(card.w.is_finite() && card.h.is_finite());
                assert!(card.x >= -0.1 && card.y >= -0.1);
                assert!(card.x + card.w <= width + 0.5);
                assert!(card.y + card.h <= height + 0.5);
                let Some((flown, _)) = fly_cover(origin(), card, step, width, height) else {
                    continue;
                };
                assert!(flown.w > 1.0 && flown.h > 1.0);
                assert!(
                    (flown.w / flown.h - aspect).abs() < 0.001,
                    "{}",
                    flown.w / flown.h
                );
                assert!(flown.x >= -0.1 && flown.y >= -0.1);
                assert!(flown.x + flown.w <= width + 0.5);
                assert!(flown.y + flown.h <= height + 0.5);
            }
        }
        let card = lerp_rect(
            shelf_card(1440.0, 900.0, 0.0),
            form::reading_frame(1440.0, 900.0, 0.0, 0.0, true).card,
            0.5,
        );
        let (flown, alpha) = fly_cover(origin(), card, 0.5, 1440.0, 900.0).unwrap();
        assert!((alpha - 1.0).abs() < 0.001);
        let center_x = flown.x + flown.w * 0.5;
        let card_x = card.x + card.w * 0.5;
        assert!((center_x - card_x).abs() < 1.0, "{center_x} {card_x}");
    }

    #[test]
    fn a_narrow_window_does_not_squeeze_the_page() {
        let covered = form::reading_frame(578.0, 528.0, 0.0, 0.0, true);
        let open = form::reading_frame(578.0, 528.0, 0.0, 0.0, false);
        assert!(covered.covers);
        assert!((covered.card.w - open.card.w).abs() < 0.01);
        let wide = form::reading_frame(1440.0, 900.0, 0.0, 0.0, true);
        let wide_open = form::reading_frame(1440.0, 900.0, 0.0, 0.0, false);
        assert!(!wide.covers);
        assert!(wide_open.card.w - wide.card.w > 200.0);
        let full = form::reading_frame(2560.0, 1440.0, 0.0, 0.0, true);
        assert!(!full.covers);
        assert!(full.card.x >= 0.0 && full.card.w > 1.0);
        assert!(full.card.x + full.card.w < 2560.0);
    }

    #[test]
    fn an_idle_pass_does_not_change_the_immersive_frame() {
        let _clean = Clean;
        reset();
        crate::ease::settle(false, true);
        let settled = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        let again = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert_eq!(settled, again);
        intend_reading(
            "a.pdf",
            Some(Rect {
                x: 300.0,
                y: 180.0,
                w: 128.0,
                h: 192.0,
            }),
        );
        let shown = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        let frame =
            crate::form::reading_frame(1440.0, 900.0, 0.0, 0.0, crate::form::margin_shown());
        assert_eq!(shown.page, frame.card);
        assert!(
            (shown.card.w - frame.card.w).abs() > 1.0 || (shown.card.x - frame.card.x).abs() > 1.0
        );
        crate::ease::settle(false, false);
        let leaving = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert_eq!(leaving.page, frame.card);
        crate::ease::retarget(true, true);
        let parked = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert!(!moving());
        crate::ease::settle(false, true);
        crate::ease::retarget(true, true);
        let clean = crate::ease::present(1440.0, 900.0, 0.0, crate::ease::current());
        assert_eq!(parked, clean);
    }
}
