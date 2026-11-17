//! Cover grid the shell paints. The mockup width keeps the measured plates.
//! A narrower card uses two 112-point covers so the second one stays inside
//! the span the headless runner showed in S2.

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

/// Columns, gaps, and cover size for a card content width in points.
pub fn shelf_grid(content_w: f32) -> ShelfGrid {
    if content_w + 0.5 >= 808.0 {
        return ShelfGrid {
            cover_w: 128.0,
            cover_h: 192.0,
            columns: 5,
            column_gap: 42.0,
            row_gap: 28.0,
            caption_h: 42.0,
            scrolls: true,
        };
    }
    // 112 + 16 + 112 ends at x 506 when the shelf origin is 266, inside 512.
    if content_w + 0.5 >= 240.0 {
        return ShelfGrid {
            cover_w: 112.0,
            cover_h: 168.0,
            columns: 2,
            column_gap: 16.0,
            row_gap: 16.0,
            caption_h: 36.0,
            scrolls: true,
        };
    }
    ShelfGrid {
        cover_w: 112.0,
        cover_h: 168.0,
        columns: 1,
        column_gap: 0.0,
        row_gap: 16.0,
        caption_h: 36.0,
        scrolls: true,
    }
}

/// Right edge of the second cover. The shelf origin is sidebar 220, inset 6, pad 40.
pub fn second_cover_right(content_w: f32) -> f32 {
    let grid = shelf_grid(content_w);
    266.0 + grid.cover_w + grid.column_gap + grid.cover_w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_headless_second_cover_stays_inside_the_span_s2_showed() {
        let content = 1280.0_f32 - 220.0 - 320.0 - 12.0 - 80.0;
        assert!((content - 648.0).abs() < 0.1, "{content}");
        let grid = shelf_grid(content);
        assert!(grid.columns >= 2, "{}", grid.columns);
        let right = second_cover_right(content);
        assert!(right <= 512.0, "second cover right {right} is past 512");
    }

    #[test]
    fn the_mockup_grid_matches_the_measured_cover() {
        let grid = shelf_grid(808.0);
        assert_eq!(grid.columns, 5);
        assert!((grid.cover_w - 128.0).abs() < 0.1);
        assert!((grid.cover_h - 192.0).abs() < 0.1);
        assert!((grid.column_gap - 42.0).abs() < 0.1);
        assert!((grid.row_gap - 28.0).abs() < 0.1);
        assert!((grid.caption_h - 42.0).abs() < 0.1);
        assert!(grid.scrolls);
    }
}
