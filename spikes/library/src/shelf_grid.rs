//! Cover grid the shell paints.
//!
//! Columns are `floor((available + gap) / (min cover + gap))`.
//! The cover stretches from 112 to 168 points to fill the row, at 2:3.
//! The gaps stay fixed. At a content width of 808 the row is five 128 by 192 covers.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShelfGrid {
    pub cover_w: f32,
    pub cover_h: f32,
    pub columns: u32,
    pub column_gap: f32,
    pub row_gap: f32,
    pub caption_h: f32,
    pub scrolls: bool,
}

pub const COVER_MIN_W: f32 = 112.0;
pub const COVER_MAX_W: f32 = 168.0;
pub const COLUMN_GAP: f32 = 42.0;
pub const ROW_GAP: f32 = 28.0;
pub const CAPTION_H: f32 = 42.0;

/// Columns, gaps, and cover size for a card content width in points.
pub fn shelf_grid(content_w: f32) -> ShelfGrid {
    let usable = if content_w.is_finite() {
        content_w.max(0.0)
    } else {
        0.0
    };
    let columns = ((usable + COLUMN_GAP) / (COVER_MIN_W + COLUMN_GAP)).floor();
    let columns = if columns >= 1.0 { columns as u32 } else { 1 };
    let raw = (usable - (columns - 1) as f32 * COLUMN_GAP) / columns as f32;
    let cover_w = if usable < COVER_MIN_W {
        usable
    } else {
        raw.clamp(0.0, COVER_MAX_W)
    };
    ShelfGrid {
        cover_w,
        cover_h: cover_w * 3.0 / 2.0,
        columns,
        column_gap: if columns <= 1 { 0.0 } else { COLUMN_GAP },
        row_gap: ROW_GAP,
        caption_h: CAPTION_H,
        scrolls: true,
    }
}

/// Width the row occupies. It is never wider than `content_w`.
pub fn row_width(grid: ShelfGrid) -> f32 {
    grid.columns as f32 * grid.cover_w + grid.columns.saturating_sub(1) as f32 * grid.column_gap
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(got: f32, want: f32) {
        assert!((got - want).abs() < 0.1, "{got} vs {want}");
    }

    #[test]
    fn the_mockup_grid_matches_the_measured_cover() {
        let grid = shelf_grid(808.0);
        assert_eq!(grid.columns, 5);
        near(grid.cover_w, 128.0);
        near(grid.cover_h, 192.0);
        near(grid.column_gap, 42.0);
        near(grid.row_gap, 28.0);
        near(grid.caption_h, 42.0);
        assert!(grid.scrolls);
        near(row_width(grid), 808.0);
    }

    #[test]
    fn the_minimum_shelf_is_two_covers() {
        let grid = shelf_grid(266.0);
        assert_eq!(grid.columns, 2);
        near(grid.cover_w, 112.0);
        near(grid.cover_h, 168.0);
        near(grid.column_gap, 42.0);
        near(row_width(grid), 266.0);
    }

    #[test]
    fn a_1280_window_uses_four_stretched_covers() {
        let grid = shelf_grid(648.0);
        assert_eq!(grid.columns, 4);
        near(grid.cover_w, 130.5);
        near(grid.cover_h, 195.75);
        near(row_width(grid), 648.0);
    }

    #[test]
    fn the_row_stays_inside_its_content_width() {
        for width in [0.0, 50.0, 200.0, 266.0, 400.0, 648.0, 808.0, 1200.0, 1920.0] {
            let grid = shelf_grid(width);
            let used = row_width(grid);
            assert!(used <= width + 0.1, "{width} used {used}");
            assert!(grid.cover_w <= COVER_MAX_W + 0.1, "{}", grid.cover_w);
            assert!((grid.cover_h - grid.cover_w * 1.5).abs() < 0.1);
            if width >= COVER_MIN_W {
                assert!(grid.cover_w + 0.1 >= COVER_MIN_W, "{width}");
            }
            assert!(grid.scrolls);
            near(grid.row_gap, ROW_GAP);
            near(grid.caption_h, CAPTION_H);
        }
    }
}
