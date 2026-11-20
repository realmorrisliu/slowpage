//! Full-form measures from the approved B mockup stylesheet.
//! Paint reads these constants. A second copy of a size does not belong in a view.

use nsh_core::color::rgba;
use nsh_core::prelude::*;
use nsh_core::BoxShadow;

pub const FONT_STACK: &str =
    "-apple-system, BlinkMacSystemFont, PingFang SC, Helvetica Neue, sans-serif";

pub const INK: u32 = 0x001D_1D1F;
pub const QUIET: u32 = 0x008E_8E93;
pub const ICON: u32 = 0x006E_6E73;
pub const FAINT: u32 = 0x00A1_A1A6;
pub const CHIP_INK: u32 = 0x003A_3A3F;
pub const CITE: u32 = 0x0055_555B;
pub const FIELD_TOP: u32 = 0x00EE_EEF0;
pub const FIELD_BOTTOM: u32 = 0x00E9_E9EC;
pub const CARD: u32 = 0x00FF_FFFF;
pub const CONTINUE_BG: u32 = 0x00F5_F5F7;
pub const SCOPE_BG: u32 = 0x00F2_F2F4;
pub const SEND_BG: u32 = 0x00D8_D8DD;
pub const AVATAR_FROM: u32 = 0x00F0_B267;
pub const AVATAR_TO: u32 = 0x00D2_693C;
pub const APPLE_FROM: u32 = 0x00FF_A53D;
pub const APPLE_TO: u32 = 0x00F2_7A1A;
pub const CALIBRE_FROM: u32 = 0x006B_5A4A;
pub const CALIBRE_TO: u32 = 0x004A_3C30;
pub const LIST_WANT: u32 = 0x00E0_773A;
pub const LIST_LU: u32 = 0x005B_8DEF;
pub const LIST_EN: u32 = 0x007D_8C6A;

pub const BODY_TRACK_EM: f32 = -0.005;
pub const H1_TRACK_EM: f32 = -0.01;
pub const LABEL_TRACK_EM: f32 = 0.04;
pub const TAG_TRACK_EM: f32 = 0.04;

pub const H1_SIZE: f32 = 26.0;
pub const H1_WEIGHT: FontWeight = FontWeight::BOLD;
pub const COUNT_SIZE: f32 = 13.0;
pub const COUNT_WEIGHT: FontWeight = FontWeight::NORMAL;
pub const SECTION_SIZE: f32 = 15.0;
pub const SECTION_WEIGHT: FontWeight = FontWeight::SEMIBOLD;
pub const SECTION_NOTE_SIZE: f32 = 12.0;
pub const BODY_SIZE: f32 = 13.0;
pub const BODY_WEIGHT: FontWeight = FontWeight::NORMAL;
pub const ROW_WEIGHT_ON: FontWeight = FontWeight::SEMIBOLD;
pub const SEARCH_SIZE: f32 = 12.5;
pub const LABEL_SIZE: f32 = 11.0;
pub const LABEL_WEIGHT: FontWeight = FontWeight::SEMIBOLD;
pub const COUNT_ROW_SIZE: f32 = 12.0;
pub const TOOL_SIZE: f32 = 12.5;
pub const SHELF_TITLE_SIZE: f32 = 12.5;
pub const SHELF_TITLE_WEIGHT: FontWeight = FontWeight::MEDIUM;
pub const SHELF_META_SIZE: f32 = 11.5;
pub const CONTINUE_TITLE_SIZE: f32 = 13.0;
pub const CONTINUE_TITLE_WEIGHT: FontWeight = FontWeight::SEMIBOLD;
pub const CONTINUE_META_SIZE: f32 = 12.0;
pub const PROGRESS_SIZE: f32 = 11.0;
pub const TIME_SIZE: f32 = 11.0;
pub const TAG_SIZE: f32 = 9.0;
pub const TAG_SM_SIZE: f32 = 8.5;
pub const TAG_WEIGHT: FontWeight = FontWeight::SEMIBOLD;
pub const NAME_SIZE: f32 = 13.0;
pub const NAME_WEIGHT: FontWeight = FontWeight::SEMIBOLD;
pub const SUB_SIZE: f32 = 11.5;
pub const AVATAR_MARK_SIZE: f32 = 13.0;
pub const AVATAR_MARK_WEIGHT: FontWeight = FontWeight::BOLD;
pub const MSG_SIZE: f32 = 13.0;
pub const CHIP_SIZE: f32 = 12.0;
pub const CITE_SIZE: f32 = 11.5;
pub const TIME_RULE_SIZE: f32 = 11.0;

pub const SIDEBAR_W: f64 = 220.0;
pub const CHAT_W: f64 = 320.0;
pub const INSET: f64 = 6.0;
/// Hidden title bar. NSH centers the traffic lights in this height.
pub const CHROME_H: f32 = 48.0;
pub const CARD_RADIUS: f64 = 12.0;
pub const WINDOW_W: f64 = 1440.0;
pub const WINDOW_H: f64 = 900.0;

pub const SIDE_PAD_X: f32 = 10.0;
pub const SIDE_PAD_TOP: f32 = 52.0;
/// The macOS window corner. The card radius plus the card inset.
/// A pinned row or a rounded card closer than this is clipped.
pub const WINDOW_CORNER: f32 = CARD_RADIUS as f32 + INSET as f32;
/// Clears the window corner. The stylesheet's 12 sits inside it.
pub const SIDE_PAD_BOTTOM: f32 = WINDOW_CORNER;
pub const TOP_ROW_H: f32 = CHROME_H;
pub const TOGGLE_SIZE: f32 = 28.0;
pub const TOGGLE_RADIUS: f32 = 7.0;
pub const TOGGLE_TOP: f32 = (CHROME_H - TOGGLE_SIZE) / 2.0;
pub const SEARCH_H: f32 = 28.0;
pub const SEARCH_RADIUS: f32 = 8.0;
pub const SEARCH_PAD_X: f32 = 8.0;
pub const SEARCH_GAP: f32 = 6.0;
pub const SEARCH_MB: f32 = 8.0;
pub const LABEL_H: f32 = 16.0;
pub const LABEL_MT: f32 = 14.0;
pub const LABEL_MB: f32 = 4.0;
pub const LABEL_PX: f32 = 10.0;
pub const ROW_H: f32 = 28.0;
pub const ROW_RADIUS: f32 = 7.0;
pub const ROW_PAD_L: f32 = 8.0;
pub const ROW_PAD_R: f32 = 10.0;
pub const ROW_GAP: f32 = 8.0;
pub const ROW_ICON_SLOT: f32 = 18.0;
pub const ROW_ICON: f32 = 16.0;
pub const DOT: f32 = 9.0;
pub const SOURCE_BADGE: f32 = 16.0;
pub const SOURCE_BADGE_RADIUS: f32 = 4.0;
pub const SOURCE_GLYPH: f32 = 11.0;
pub const ADD_ICON: f32 = 14.0;
pub const FOOT_H: f32 = 28.0;
pub const FOOT_GAP: f32 = 8.0;
pub const FOOT_PX: f32 = 8.0;
pub const SETTINGS_ICON: f32 = 16.0;

/// Card inset on both sides of the title bar, so the toolbar center is the light center.
pub const TOOLBAR_H: f32 = CHROME_H - (INSET as f32) * 2.0;
pub const TOOLBAR_PX: f32 = 12.0;
pub const TOOLBAR_GAP: f32 = 6.0;
pub const TOOL_H: f32 = 28.0;
pub const TOOL_PX: f32 = 8.0;
pub const TOOL_GAP: f32 = 5.0;
pub const TOOL_RADIUS: f32 = 7.0;
pub const SEG_PAD: f32 = 2.0;
pub const SEG_RADIUS: f32 = 8.0;
pub const SEG_BTN_W: f32 = 28.0;
pub const SEG_BTN_H: f32 = 22.0;
pub const SEG_BTN_RADIUS: f32 = 6.0;
pub const SORT_ICON: f32 = 15.0;
pub const CHEVRON_ICON: f32 = 11.0;
pub const VIEW_ICON: f32 = 15.0;
pub const SIDEBAR_ICON: f32 = 18.0;
pub const PANEL_ICON: f32 = 18.0;
pub const IMMERSIVE_ICON: f32 = 16.0;
pub const CARD_SPREAD: f32 = 0.5;
pub const CARD_SPREAD_ALPHA: f32 = 0.07;

pub const CARD_PAD_X: f32 = 40.0;
pub const CARD_PAD_TOP: f32 = 4.0;
pub const TITLE_ROW_H: f32 = 36.0;
pub const TITLE_GAP: f32 = 10.0;
pub const SECTION_MT: f32 = 28.0;
pub const SECTION_MB: f32 = 12.0;
pub const SECTION_GAP: f32 = 8.0;

pub const CONTINUE_GAP: f32 = 16.0;
pub const CONTINUE_MAX: usize = 3;
pub const CONTINUE_PAD: f32 = 12.0;
pub const CONTINUE_RADIUS: f32 = 12.0;
pub const CONTINUE_TEXT_PT: f32 = 0.0;
pub const CONTINUE_META_MT: f32 = 2.0;
pub const CONTINUE_INNER_GAP: f32 = 14.0;
pub const CONTINUE_COVER_W: f32 = 52.0;
pub const CONTINUE_COVER_H: f32 = 78.0;
pub const BAR_H: f32 = 3.0;
pub const BAR_RADIUS: f32 = 2.0;
pub const BAR_GAP: f32 = 8.0;

pub const SHELF_COVER_W: f32 = 128.0;
pub const SHELF_COVER_H: f32 = 192.0;
pub const COVER_LABEL_PX: f32 = 8.0;
pub const SHELF_RADIUS: f32 = 3.0;
pub const SHELF_ROW_GAP: f32 = 28.0;
/// Sidebar, two minimum covers, and the chrome around them. Chat is hidden.
pub const MIN_WINDOW_W: f64 = 578.0;
/// Two 112-point covers and the 42-point gap. The content width of the minimum window.
pub const TWO_COLUMN_CONTENT: f32 = 266.0;
/// Minimum window plus the 320-point chat panel, with the sidebar shown.
pub const CHAT_BREAKPOINT: f64 = 898.0;
pub const SHELF_TITLE_MT: f32 = 10.0;
pub const SHELF_META_MT: f32 = 1.0;
pub const SHELF_META_GAP: f32 = 6.0;

pub const TAG_RADIUS: f32 = 3.0;
pub const TAG_PX: f32 = 3.0;
pub const TAG_BORDER: f32 = 0.5;

pub const CHAT_PAD_X: f32 = 16.0;
pub const MARGIN_CARD_W: f32 = 208.0;
pub const MARGIN_CARD_X: f32 = CHAT_PAD_X;
pub const MARGIN_CARD_GAP: f32 = ROW_GAP;
pub const MARGIN_COMPOSER_H: f32 = 88.0;
pub const CHAT_HEAD_H: f32 = CHROME_H;
pub const CHAT_HEAD_GAP: f32 = 10.0;
pub const AVATAR: f32 = 26.0;
pub const NAME_GAP: f32 = 6.0;
pub const TOOL_CLUSTER_GAP: f32 = 2.0;
pub const CONV_GAP: f32 = 14.0;
pub const CONV_PT: f32 = 12.0;
pub const BUBBLE_RADIUS: f32 = 14.0;
pub const BUBBLE_TAIL: f32 = 4.0;
pub const BUBBLE_PX: f32 = 12.0;
pub const BUBBLE_PY: f32 = 8.0;
pub const BUBBLE_MAX: f32 = 250.0;
pub const COMPOSER_CLEAR: f32 = 120.0;
/// The composer card clears the window corner on every side.
pub const COMPOSER_INSET: f32 = WINDOW_CORNER;
pub const COMPOSER_RADIUS: f32 = 14.0;
/// One pad inside the composer. The stylesheet's 12, 10, 8, and 14 do not share an edge.
pub const COMPOSER_PAD: f32 = 12.0;
pub const COMPOSER_PT: f32 = COMPOSER_PAD;
pub const COMPOSER_PR: f32 = COMPOSER_PAD;
pub const COMPOSER_PB: f32 = COMPOSER_PAD;
pub const COMPOSER_PL: f32 = COMPOSER_PAD;
pub const PLACEHOLDER_H: f32 = 20.0;
pub const COMPOSER_ROW_MT: f32 = 20.0;
pub const COMPOSER_ROW_GAP: f32 = 6.0;
pub const ATTACH: f32 = 24.0;
pub const ATTACH_ICON: f32 = 16.0;
pub const SCOPE_PX: f32 = 7.0;
/// The chip and the send button are one control, so the row has one center and one height.
pub const COMPOSER_CONTROL: f32 = 26.0;
#[allow(dead_code)]
pub const SCOPE_H: f32 = COMPOSER_CONTROL;
pub const SCOPE_RADIUS: f32 = 6.0;
pub const SCOPE_GAP: f32 = 4.0;
pub const SEND: f32 = COMPOSER_CONTROL;
pub const SEND_ICON: f32 = 14.0;
pub const CHIP_RADIUS: f32 = 999.0;
pub const CHIP_GAP: f32 = 6.0;
pub const CHIP_PX: f32 = 10.0;
pub const CHIP_PY: f32 = 4.0;
pub const ROW_ON_ALPHA: f32 = 0.07;
pub const SEARCH_ALPHA: f32 = 0.05;
pub const SEG_ALPHA: f32 = 0.05;
pub const BAR_TRACK_ALPHA: f32 = 0.08;
pub const TAG_BORDER_ALPHA: f32 = 0.25;

pub fn track(size: f32, em: f32) -> f32 {
    size * em
}

/// Vertical center of the traffic lights, measured from the top of the window.
pub fn chrome_center() -> f32 {
    CHROME_H / 2.0
}

/// Top of a control whose center is the traffic-light center.
pub fn chrome_top(size: f32) -> f32 {
    (CHROME_H - size) / 2.0
}

/// One standard window button, in top-left window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub shown: bool,
}

/// A button counts as shown while it is drawn above half alpha.
pub fn light_alpha_shown(alpha: f64) -> bool {
    alpha > 0.5
}

/// The buttons are on screen only when each one is shown and sits in the title-bar band.
/// A frame parked at the bottom of a fullscreen window is not that band.
pub fn lights_shown(buttons: &[LightBox], window_w: f32, window_h: f32) -> bool {
    if buttons.len() < 3 || window_w < 1.0 || window_h < 1.0 {
        return false;
    }
    buttons.iter().all(|button| {
        button.shown
            && button.w >= 1.0
            && button.h >= 1.0
            && button.x >= -0.5
            && button.x + button.w <= window_w + 0.5
            && button.y >= -0.5
            && button.y + button.h <= CHROME_H + 0.5
    })
}

/// What the shell row needs from the live window. `bar` is the measured light span.
/// `top` is the content view's distance below the window top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromeIn {
    pub lights: bool,
    pub bar: f32,
    pub top: f32,
}

/// The header row. One value for the toggle, the search, and the card toolbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromeRow {
    pub height: f32,
    pub toggle_x: f32,
    pub toggle_y: f32,
    pub card_y: f32,
    pub toolbar_h: f32,
}

pub fn chrome_row(input: ChromeIn) -> ChromeRow {
    let top = input.top.max(0.0);
    let lights = input.lights && input.bar > 8.0;
    let pad = if lights { TOGGLE_TOP } else { SIDE_PAD_X };
    ChromeRow {
        height: top + pad + TOGGLE_SIZE + pad,
        toggle_x: if lights { input.bar } else { SIDE_PAD_X },
        toggle_y: top + pad,
        card_y: top + INSET as f32,
        toolbar_h: TOOLBAR_H,
    }
}

/// How far the content view sits below the top of the window.
///
/// The inset counts only when the layout is as tall as the window, so a gap
/// above a shorter content view is a real band in layout coordinates. A layout
/// that is already the content view's height is not shifted again. A gap
/// outside the title-bar band is ignored.
pub fn content_top(window_h: f64, content_h: f64, content_y: f64, stage_h: f64) -> f64 {
    let top = window_h - (content_y + content_h);
    let layout_includes_the_gap = (stage_h - window_h).abs() <= 1.0;
    if layout_includes_the_gap && top.is_finite() && (0.5..=f64::from(CHROME_H)).contains(&top) {
        top
    } else {
        0.0
    }
}

/// Card content width after the live sidebar, the live chat panel, the insets, and the pads.
pub fn content_width(window_w: f32, sidebar_w: f32, chat_w: f32) -> f32 {
    let card = window_w - sidebar_w - chat_w - INSET as f32 * 2.0;
    (card - CARD_PAD_X * 2.0).max(0.0)
}

/// The 320-point chat panel fits beside a two-column shelf.
pub fn chat_fits(window_w: f32, sidebar_w: f32) -> bool {
    content_width(window_w, sidebar_w, CHAT_W as f32) + 0.5 >= TWO_COLUMN_CONTENT
}

/// One section label: the margin above it, the line box, and the margin below it.
/// Text paints a line box of 1.2 times the font size.
pub fn section_block() -> f32 {
    SECTION_MT + SECTION_SIZE * 1.2 + SECTION_MB
}

/// A continue card is the 78-point cover plus the padding above and below it.
pub fn continue_card_h() -> f32 {
    CONTINUE_PAD * 2.0 + CONTINUE_COVER_H
}

/// Toolbar, title, the continue row, and one full shelf row, with both section labels.
/// The last `CARD_RADIUS` is empty card, so the caption clears the rounded corner.
pub fn library_stack() -> f32 {
    let shelf = spike_library::shelf_grid(TWO_COLUMN_CONTENT);
    TOOLBAR_H
        + CARD_PAD_TOP
        + TITLE_ROW_H
        + section_block()
        + continue_card_h()
        + section_block()
        + shelf.cover_h
        + shelf.caption_h
        + CARD_RADIUS as f32
}

/// The stack plus the inset above and below the card.
pub fn library_min_height() -> f64 {
    f64::from(library_stack()) + INSET * 2.0
}

pub fn window_min() -> Size<Points> {
    size(px(MIN_WINDOW_W as f32), px(library_min_height() as f32))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinueLayout {
    pub cards: u32,
    pub card_w: f32,
    pub cover_w: f32,
    pub cover_h: f32,
    pub gap: f32,
}

/// Continue cards share the row, then the count drops. The cover stays 52 by 78.
pub fn continue_layout(content_w: f32, available: u32) -> ContinueLayout {
    let content_w = if content_w.is_finite() {
        content_w.max(0.0)
    } else {
        0.0
    };
    let wanted = available.min(CONTINUE_MAX as u32);
    if wanted == 0 {
        return ContinueLayout {
            cards: 0,
            card_w: 0.0,
            cover_w: CONTINUE_COVER_W,
            cover_h: CONTINUE_COVER_H,
            gap: 0.0,
        };
    }
    let card_min = CONTINUE_PAD * 2.0 + CONTINUE_COVER_W + CONTINUE_INNER_GAP + CONTINUE_TITLE_SIZE;
    let mut cards = wanted;
    while cards > 1 {
        let need = cards as f32 * card_min + (cards - 1) as f32 * CONTINUE_GAP;
        if need <= content_w + 0.5 {
            break;
        }
        cards -= 1;
    }
    let gap = if cards <= 1 { 0.0 } else { CONTINUE_GAP };
    let card_w = ((content_w - (cards - 1) as f32 * gap) / cards as f32).max(0.0);
    ContinueLayout {
        cards,
        card_w,
        cover_w: CONTINUE_COVER_W,
        cover_h: CONTINUE_COVER_H,
        gap,
    }
}

fn black(alpha: f32) -> nsh_core::color::Rgba {
    let byte = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    rgba(byte)
}

fn layer(x: f32, y: f32, blur: f32, spread: f32, alpha: f32) -> BoxShadow {
    BoxShadow {
        offset_x: px(x),
        offset_y: px(y),
        blur: px(blur),
        spread: px(spread),
        color: black(alpha).into(),
        inset: false,
    }
}

pub fn card_shadows() -> Vec<BoxShadow> {
    vec![
        layer(0.0, 0.0, 0.0, CARD_SPREAD, CARD_SPREAD_ALPHA),
        layer(0.0, 1.0, 2.0, 0.0, 0.04),
        layer(0.0, 6.0, 20.0, 0.0, 0.045),
    ]
}

pub fn margin_ring() -> Vec<BoxShadow> {
    vec![layer(0.0, 0.0, 0.0, CARD_SPREAD, CARD_SPREAD_ALPHA)]
}

pub fn cover_shadows() -> Vec<BoxShadow> {
    vec![
        layer(0.0, 0.0, 0.0, 0.5, 0.08),
        layer(0.0, 1.0, 2.0, 0.0, 0.12),
        layer(0.0, 6.0, 14.0, -4.0, 0.18),
    ]
}

pub fn bubble_shadows() -> Vec<BoxShadow> {
    vec![
        layer(0.0, 0.0, 0.0, 0.5, 0.05),
        layer(0.0, 1.0, 2.0, 0.0, 0.04),
    ]
}

pub fn composer_shadows() -> Vec<BoxShadow> {
    vec![
        layer(0.0, 0.0, 0.0, 0.5, 0.08),
        layer(0.0, 2.0, 10.0, 0.0, 0.05),
    ]
}

pub fn seg_shadows() -> Vec<BoxShadow> {
    vec![
        layer(0.0, 0.0, 0.0, 0.5, 0.06),
        layer(0.0, 1.0, 2.0, 0.0, 0.08),
    ]
}

pub fn chip_shadows() -> Vec<BoxShadow> {
    vec![layer(0.0, 0.0, 0.0, 0.5, 0.12)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mockup_content_width_is_808() {
        let content = content_width(WINDOW_W as f32, SIDEBAR_W as f32, CHAT_W as f32);
        assert!((content - 808.0).abs() < 0.1, "{content}");
    }

    #[test]
    fn the_chat_panel_hides_below_the_two_column_shelf() {
        assert!(chat_fits(CHAT_BREAKPOINT as f32, SIDEBAR_W as f32));
        assert!(!chat_fits((CHAT_BREAKPOINT - 1.0) as f32, SIDEBAR_W as f32));
        assert!(
            (content_width(MIN_WINDOW_W as f32, SIDEBAR_W as f32, 0.0) - TWO_COLUMN_CONTENT).abs()
                < 0.1
        );
        let wide = continue_layout(808.0, 3);
        assert_eq!(wide.cards, 3);
        assert!((wide.cover_w - 52.0).abs() < 0.1);
        let narrow = continue_layout(TWO_COLUMN_CONTENT, 3);
        assert_eq!(narrow.cards, 2);
        assert!((narrow.cover_w - 52.0).abs() < 0.1);
    }

    #[test]
    fn the_minimum_window_fits_the_continue_row_and_one_shelf_row() {
        let stack = library_stack();
        let shelf = spike_library::shelf_grid(TWO_COLUMN_CONTENT);
        let continue_and_shelf =
            section_block() + continue_card_h() + section_block() + shelf.cover_h + shelf.caption_h;
        assert!(stack > continue_and_shelf);
        assert!((library_min_height() - (f64::from(stack) + INSET * 2.0)).abs() < 0.1);
        assert!(library_min_height() > 400.0);
    }

    #[test]
    fn margin_card_tokens_match_the_layout() {
        assert!((f64::from(MARGIN_CARD_W) - spike_library::margin::CARD_W).abs() < 0.01);
        assert!((f64::from(MARGIN_CARD_X) - spike_library::margin::CARD_X).abs() < 0.01);
        assert!((f64::from(MARGIN_CARD_GAP) - spike_library::margin::GAP).abs() < 0.01);
        assert!((CARD_RADIUS - spike_library::margin::RADIUS).abs() < 0.01);
        assert!((f64::from(MARGIN_COMPOSER_H) - spike_library::margin::COMPOSER_H).abs() < 0.01);
        assert!((f64::from(COMPOSER_INSET) - spike_library::margin::COMPOSER_INSET).abs() < 0.01);
        assert_eq!(margin_ring().len(), 1);
    }

    #[test]
    fn the_chrome_row_shares_the_traffic_light_center() {
        let center = chrome_center();
        assert!((center - 24.0).abs() < 0.01, "{center}");
        assert!((TOP_ROW_H / 2.0 - center).abs() < 0.01);
        assert!((CHAT_HEAD_H / 2.0 - center).abs() < 0.01);
        assert!((TOGGLE_TOP + TOGGLE_SIZE / 2.0 - center).abs() < 0.01);
        assert!((INSET as f32 + TOOLBAR_H / 2.0 - center).abs() < 0.01);
        assert!((library_min_height() - 528.0).abs() < 0.1);
        assert!((WINDOW_CORNER - 18.0).abs() < 0.01);
        assert!((COMPOSER_INSET - WINDOW_CORNER).abs() < 0.01);
        assert!((SIDE_PAD_BOTTOM - WINDOW_CORNER).abs() < 0.01);
        assert!((COMPOSER_PT - COMPOSER_PAD).abs() < 0.01);
        assert!((COMPOSER_PR - COMPOSER_PAD).abs() < 0.01);
        assert!((COMPOSER_PB - COMPOSER_PAD).abs() < 0.01);
        assert!((COMPOSER_PL - COMPOSER_PAD).abs() < 0.01);
        assert!((SCOPE_H - SEND).abs() < 0.01);
        let shown = chrome_row(ChromeIn {
            lights: true,
            bar: 78.0,
            top: 0.0,
        });
        assert!((shown.toggle_x - 78.0).abs() < 0.01);
        assert!((shown.toggle_y + TOGGLE_SIZE / 2.0 - center).abs() < 0.01);
        assert!((shown.card_y + shown.toolbar_h / 2.0 - center).abs() < 0.01);
        let hidden = chrome_row(ChromeIn {
            lights: false,
            bar: 78.0,
            top: 0.0,
        });
        assert!((hidden.toggle_x - SIDE_PAD_X).abs() < 0.01);
        assert!((hidden.toggle_y + TOGGLE_SIZE / 2.0 - center).abs() < 0.01);
        assert!((hidden.height - shown.height).abs() < 0.01);
        let dropped = chrome_row(ChromeIn {
            lights: false,
            bar: 78.0,
            top: 20.0,
        });
        assert!((dropped.card_y - (20.0 + INSET as f32)).abs() < 0.01);
        assert!((dropped.toggle_y - 30.0).abs() < 0.01);
        assert!(
            (dropped.toggle_y + TOGGLE_SIZE / 2.0 - (dropped.card_y + dropped.toolbar_h / 2.0))
                .abs()
                < 0.01
        );
        let windowed = [
            LightBox {
                x: 9.0,
                y: 17.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
            LightBox {
                x: 32.0,
                y: 17.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
            LightBox {
                x: 55.0,
                y: 17.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
        ];
        assert!(lights_shown(&windowed, 1440.0, 855.0));
        let parked = [
            LightBox {
                x: 9.0,
                y: 894.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
            LightBox {
                x: 32.0,
                y: 894.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
            LightBox {
                x: 55.0,
                y: 894.0,
                w: 14.0,
                h: 14.0,
                shown: true,
            },
        ];
        assert!(!lights_shown(&parked, 1512.0, 949.0));
        let faded = [
            LightBox {
                x: 9.0,
                y: 17.0,
                w: 14.0,
                h: 14.0,
                shown: false,
            },
            windowed[1],
            windowed[2],
        ];
        assert!(!lights_shown(&faded, 1440.0, 855.0));
        assert!((content_top(949.0, 949.0, 0.0, 949.0)).abs() < 0.01);
        assert!((content_top(949.0, 925.0, 0.0, 949.0) - 24.0).abs() < 0.01);
        assert!((content_top(949.0, 925.0, 0.0, 925.0)).abs() < 0.01);
        assert!((content_top(949.0, 100.0, 0.0, 949.0)).abs() < 0.01);
    }

    #[test]
    fn a_short_fullscreen_stage_is_not_converted_into_an_inset() {
        // The content view already fills the 949pt window. The old 855pt
        // stage is not a title bar, and neither is the 45pt menu bar.
        assert!(content_top(949.0, 949.0, 0.0, 855.0).abs() < 0.01);
        assert!(content_top(949.0, 855.0, 0.0, 855.0).abs() < 0.01);
        assert!(content_top(855.0, 855.0, 0.0, 855.0).abs() < 0.01);
        assert!((content_top(949.0, 925.0, 0.0, 949.0) - 24.0).abs() < 0.01);
    }
}
