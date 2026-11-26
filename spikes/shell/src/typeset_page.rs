//! Glyph-run page for `SPIKE_TYPESET=1`.
//!
//! The view paints the typesetter's raster of the glyph runs, then the
//! caret bar and the selection rects the crate already returns. The paper
//! is Books-white and the ink is the recorded `#1c1915`. A missing face or
//! a chapter the crate cannot open leaves the page on WKWebView.

use std::path::Path;

use superreader_typeset::{
    load_chapter, BreakTune, Caret, CaretRange, ChapterInput, ChapterLayout, PageRaster, Point,
    SelectionRect, Typesetter, INK_RGB, PAPERBACK_HEIGHT, PAPERBACK_WIDTH,
};

pub const TYPESET_SWITCH: &str = "SPIKE_TYPESET";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageEngine {
    WebKit,
    Typeset,
}

/// The debug switch opens the typesetter only when that open succeeds.
pub fn page_engine(debug: bool, opened: bool) -> PageEngine {
    if debug && opened {
        PageEngine::Typeset
    } else {
        PageEngine::WebKit
    }
}

pub fn debug_requested(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), Some("1"))
}

#[derive(Clone, Copy, Debug)]
pub struct PageFrame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The raster's rectangle inside `outer`, centered, at the raster's own aspect.
///
/// The typesetter paints a paperback page. A trim or a declared page with
/// another aspect letterboxes that raster. It does not stretch the glyphs.
pub fn contained_frame(outer: PageFrame, raster_w: f32, raster_h: f32) -> PageFrame {
    if outer.w <= 0.0 || outer.h <= 0.0 || raster_w <= 0.0 || raster_h <= 0.0 {
        return outer;
    }
    let page = f64::from(raster_w) / f64::from(raster_h);
    let (w, h) = if outer.w / outer.h > page {
        (outer.h * page, outer.h)
    } else {
        (outer.w, outer.w / page)
    };
    PageFrame {
        x: outer.x + (outer.w - w) * 0.5,
        y: outer.y + (outer.h - h) * 0.5,
        w,
        h,
    }
}

/// A stage point inside the fitted page, in the layout's top-down space.
pub fn layout_point(frame: PageFrame, x: f64, y: f64) -> Option<Point> {
    if !crate::visit::point_in_box(x, y, frame.x, frame.y, frame.w, frame.h) {
        return None;
    }
    if frame.w <= 0.0 || frame.h <= 0.0 {
        return None;
    }
    Some(Point {
        x: ((x - frame.x) / frame.w) as f32 * PAPERBACK_WIDTH,
        y: ((y - frame.y) / frame.h) as f32 * PAPERBACK_HEIGHT,
    })
}

/// One chapter page. The ends stay put. The count does not wrap.
pub fn turned_page(page: u32, count: u32, delta: i32) -> u32 {
    if count == 0 {
        return 0;
    }
    let last = count.saturating_sub(1) as i32;
    (page as i32 + delta).clamp(0, last) as u32
}

struct FacePaths {
    latin: String,
    italic: String,
    cjk: String,
    latin_index: u32,
    italic_index: u32,
    cjk_index: u32,
}

pub struct Session {
    setter: Typesetter,
    layout: ChapterLayout,
    title: String,
    declared: Option<(f64, f64)>,
    page: u32,
    caret: Option<Caret>,
    range: Option<CaretRange>,
    clean: Option<(u32, PageRaster)>,
}

impl Session {
    pub fn open(epub_path: &str, href: &str) -> Result<Self, String> {
        let faces = face_paths().ok_or_else(|| {
            "typesetter faces are missing (set SPIKE_TYPESET_LATIN, SPIKE_TYPESET_ITALIC, and SPIKE_TYPESET_CJK, or run just fixtures-fonts)".to_string()
        })?;
        let packed = spike_epub::pack(epub_path)?;
        let loaded = open_chapter(epub_path, href, &packed.href)?;
        let setter = Typesetter::open_at(
            &faces.latin,
            faces.latin_index,
            &faces.italic,
            faces.italic_index,
            &faces.cjk,
            faces.cjk_index,
        )
        .map_err(|err| err.to_string())?;
        let layout = setter
            .compose(ChapterInput {
                xhtml: &loaded.xhtml,
                stylesheet: &loaded.stylesheet,
                base_href: &loaded.href,
                measure: None,
                font_size: None,
                tune: BreakTune::default(),
                bold_word_starts: false,
            })
            .map_err(|err| err.to_string())?;
        Ok(Self {
            setter,
            layout,
            title: packed.title,
            declared: packed.declared_page,
            page: 0,
            caret: None,
            range: None,
            clean: None,
        })
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn page_count(&self) -> u32 {
        self.layout.report().page_count.max(1)
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn declared(&self) -> Option<(f64, f64)> {
        self.declared
    }

    pub fn turn(&mut self, delta: i32) {
        let next = turned_page(self.page, self.page_count(), delta);
        if next != self.page {
            self.page = next;
            self.caret = None;
            self.range = None;
            self.clean = None;
        }
    }

    pub fn place_caret(&mut self, point: Point) {
        self.caret = self.layout.hit_test(self.page, point);
        self.range = None;
    }

    pub fn select_span(&mut self, start: Point, end: Point) {
        let Some(from) = self.layout.hit_test(self.page, start) else {
            return;
        };
        let Some(to) = self.layout.hit_test(self.page, end) else {
            return;
        };
        if from == to {
            self.caret = Some(from);
            self.range = None;
            return;
        }
        self.range = Some(CaretRange {
            start: from,
            end: to,
        });
        self.caret = Some(to);
    }

    /// The clean page, with a drag preview or the committed caret and selection.
    pub fn marked(&mut self, preview: Option<(Point, Point)>) -> Option<PageRaster> {
        let mut image = self.clean_page()?;
        let rects = self.mark_rects(preview);
        for rect in rects {
            tint_rect(&mut image, rect);
        }
        if preview.is_none() {
            if let Some(caret) = self.caret {
                if let Some(bar) = self.layout.caret_bar(self.page, caret) {
                    ink_rect(&mut image, bar);
                }
            }
        }
        Some(image)
    }

    fn mark_rects(&self, preview: Option<(Point, Point)>) -> Vec<SelectionRect> {
        let range = if let Some((start, end)) = preview {
            match (
                self.layout.hit_test(self.page, start),
                self.layout.hit_test(self.page, end),
            ) {
                (Some(start), Some(end)) if start != end => CaretRange { start, end },
                _ => return Vec::new(),
            }
        } else if let Some(range) = self.range {
            range
        } else {
            return Vec::new();
        };
        self.layout.selection_rects(self.page, range)
    }

    fn clean_page(&mut self) -> Option<PageRaster> {
        if self
            .clean
            .as_ref()
            .is_some_and(|(page, _)| *page == self.page)
        {
            return self.clean.as_ref().map(|(_, image)| image.clone());
        }
        let image = self.setter.raster(&self.layout, self.page)?;
        self.clean = Some((self.page, image.clone()));
        Some(image)
    }
}

fn open_chapter(
    epub_path: &str,
    href: &str,
    fallback: &str,
) -> Result<superreader_typeset::LoadedChapter, String> {
    let requested = href_suffix(href);
    if !requested.is_empty() {
        if let Ok(chapter) = load_chapter(epub_path, requested) {
            return Ok(chapter);
        }
    }
    load_chapter(epub_path, href_suffix(fallback)).map_err(|err| err.to_string())
}

fn href_suffix(href: &str) -> &str {
    href.split(['#', '?'])
        .next()
        .unwrap_or(href)
        .trim_start_matches('/')
}

fn face_paths() -> Option<FacePaths> {
    if let Some(paths) = env_faces() {
        if files_ok(&paths) {
            return Some(paths);
        }
    }
    if let Some(paths) = fixture_faces() {
        return Some(paths);
    }
    reader_faces()
}

fn env_faces() -> Option<FacePaths> {
    let latin = env_path("SPIKE_TYPESET_LATIN")?;
    let italic = env_path("SPIKE_TYPESET_ITALIC")?;
    let cjk = env_path("SPIKE_TYPESET_CJK")?;
    Some(FacePaths {
        latin,
        italic,
        cjk,
        latin_index: env_index("SPIKE_TYPESET_LATIN_INDEX"),
        italic_index: env_index("SPIKE_TYPESET_ITALIC_INDEX"),
        cjk_index: env_index("SPIKE_TYPESET_CJK_INDEX"),
    })
}

fn env_path(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_index(key: &str) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

fn fixture_faces() -> Option<FacePaths> {
    let fixtures = spike_report::fixtures_dir();
    let dir = Path::new(&fixtures);
    let latin = dir.join("EBGaramond-Regular.ttf");
    let italic = dir.join("EBGaramond-Italic.ttf");
    let cjk = dir.join("NotoSerifCJKsc-Regular.otf");
    if !latin.is_file() || !italic.is_file() || !cjk.is_file() {
        return None;
    }
    Some(FacePaths {
        latin: latin.display().to_string(),
        italic: italic.display().to_string(),
        cjk: cjk.display().to_string(),
        latin_index: 0,
        italic_index: 0,
        cjk_index: 0,
    })
}

fn files_ok(paths: &FacePaths) -> bool {
    Path::new(&paths.latin).is_file()
        && Path::new(&paths.italic).is_file()
        && Path::new(&paths.cjk).is_file()
}

#[cfg(target_os = "macos")]
fn reader_faces() -> Option<FacePaths> {
    let latin = system_face(
        &["Iowan Old Style.ttc", "IowanOldStyle.ttc"],
        &["iowan"],
        &["italic", "bold"],
    )?;
    let italic = system_face(
        &["Iowan Old Style.ttc", "IowanOldStyle.ttc"],
        &["iowan", "italic"],
        &["bold"],
    )?;
    let cjk = system_face(
        &["Songti.ttc", "Songti SC.ttc", "STSongti.ttc"],
        &["songti"],
        &["bold"],
    )?;
    Some(FacePaths {
        latin: latin.0,
        latin_index: latin.1,
        italic: italic.0,
        italic_index: italic.1,
        cjk: cjk.0,
        cjk_index: cjk.1,
    })
}

#[cfg(not(target_os = "macos"))]
fn reader_faces() -> Option<FacePaths> {
    None
}

#[cfg(target_os = "macos")]
fn system_face(names: &[&str], accept: &[&str], reject: &[&str]) -> Option<(String, u32)> {
    let roots = [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
    ];
    for root in roots {
        for name in names {
            let path = Path::new(root).join(name);
            if !path.is_file() {
                continue;
            }
            let text = path.display().to_string();
            if let Some(index) = pick_index(&text, accept, reject) {
                return Some((text, index));
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn pick_index(path: &str, accept: &[&str], reject: &[&str]) -> Option<u32> {
    let faces = superreader_typeset::list_faces(path).ok()?;
    let mut chosen: Option<(u32, bool)> = None;
    for (index, name) in faces {
        let low = name.to_lowercase();
        if reject.iter().any(|word| low.contains(word)) {
            continue;
        }
        if !accept.iter().all(|word| low.contains(word)) {
            continue;
        }
        let regular = low.contains("regular");
        let replace = match chosen {
            None => true,
            Some((_, was)) => regular && !was,
        };
        if replace {
            chosen = Some((index, regular));
        }
    }
    chosen.map(|(index, _)| index)
}

/// The PDF highlight in `spike_pdf::tint_under_ink`. Paper moves toward that
/// yellow. A dark pixel stays, so the ink is still the glyph.
fn tint_rect(image: &mut PageRaster, rect: SelectionRect) {
    paint_rect(image, rect, |px| {
        let lum = px[0] as u16 + px[1] as u16 + px[2] as u16;
        if lum < 540 {
            px[3] = 255;
            return;
        }
        px[0] = ((px[0] as u16 * 2 + 255 * 8) / 10) as u8;
        px[1] = ((px[1] as u16 * 2 + 210 * 8) / 10) as u8;
        px[2] = ((px[2] as u16 * 2 + 40 * 8) / 10) as u8;
        px[3] = 255;
    });
}

fn ink_rect(image: &mut PageRaster, rect: SelectionRect) {
    paint_rect(image, rect, |px| {
        px[0] = INK_RGB.0;
        px[1] = INK_RGB.1;
        px[2] = INK_RGB.2;
        px[3] = 255;
    });
}

fn paint_rect(image: &mut PageRaster, rect: SelectionRect, paint: impl Fn(&mut [u8])) {
    if image.width == 0 || image.height == 0 {
        return;
    }
    let x0 = rect.x.floor().max(0.0) as u32;
    let y0 = rect.y.floor().max(0.0) as u32;
    let x1 = (rect.x + rect.width).ceil().max(0.0) as u32;
    let y1 = (rect.y + rect.height).ceil().max(0.0) as u32;
    let x1 = x1.min(image.width);
    let y1 = y1.min(image.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * image.width + x) * 4) as usize;
            if i + 4 <= image.rgba.len() {
                paint(&mut image.rgba[i..i + 4]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use superreader_typeset::INK_RGB;

    #[test]
    fn the_switch_stays_on_webkit_until_the_typesetter_opens() {
        assert!(!debug_requested(None));
        assert!(!debug_requested(Some("")));
        assert!(!debug_requested(Some("0")));
        assert!(debug_requested(Some("1")));
        assert!(debug_requested(Some(" 1 ")));
        assert_eq!(page_engine(false, true), PageEngine::WebKit);
        assert_eq!(page_engine(true, false), PageEngine::WebKit);
        assert_eq!(page_engine(true, true), PageEngine::Typeset);
    }

    #[test]
    fn a_turn_stops_at_the_chapter_ends() {
        assert_eq!(turned_page(0, 0, 1), 0);
        assert_eq!(turned_page(0, 4, -1), 0);
        assert_eq!(turned_page(3, 4, 1), 3);
        assert_eq!(turned_page(1, 4, -1), 0);
        assert_eq!(turned_page(1, 4, 2), 3);
    }

    #[test]
    fn a_point_maps_into_the_paperback_page() {
        let frame = PageFrame {
            x: 100.0,
            y: 40.0,
            w: 534.0,
            h: 819.0,
        };
        let origin = layout_point(frame, 100.0, 40.0).unwrap();
        assert!(origin.x.abs() < 1e-3 && origin.y.abs() < 1e-3);
        let far = layout_point(frame, 634.0, 859.0).unwrap();
        assert!((far.x - PAPERBACK_WIDTH).abs() < 1e-2);
        assert!((far.y - PAPERBACK_HEIGHT).abs() < 1e-2);
        assert!(layout_point(frame, 99.0, 40.0).is_none());
    }

    #[test]
    fn a_raster_keeps_its_aspect_inside_a_wider_page() {
        let outer = PageFrame {
            x: 10.0,
            y: 20.0,
            w: 819.0,
            h: 819.0,
        };
        let fitted = contained_frame(outer, PAPERBACK_WIDTH, PAPERBACK_HEIGHT);
        let aspect = f64::from(PAPERBACK_WIDTH) / f64::from(PAPERBACK_HEIGHT);
        assert!((fitted.h - outer.h).abs() < 1e-6);
        assert!((fitted.w / fitted.h - aspect).abs() < 1e-9);
        assert!((fitted.x - (outer.x + (outer.w - fitted.w) * 0.5)).abs() < 1e-6);
        assert!((fitted.y - outer.y).abs() < 1e-6);
        let same = contained_frame(
            PageFrame {
                x: 0.0,
                y: 0.0,
                w: f64::from(PAPERBACK_WIDTH),
                h: f64::from(PAPERBACK_HEIGHT),
            },
            PAPERBACK_WIDTH,
            PAPERBACK_HEIGHT,
        );
        assert!((same.w - f64::from(PAPERBACK_WIDTH)).abs() < 1e-6);
        assert!((same.h - f64::from(PAPERBACK_HEIGHT)).abs() < 1e-6);
    }

    #[test]
    fn selection_tints_paper_and_the_caret_is_ink() {
        let mut image = PageRaster {
            width: 4,
            height: 2,
            rgba: vec![255, 255, 255, 255, 10, 10, 10, 255]
                .into_iter()
                .cycle()
                .take(4 * 2 * 4)
                .collect(),
        };
        tint_rect(
            &mut image,
            SelectionRect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 1.0,
            },
        );
        assert_eq!(&image.rgba[0..4], &[255, 219, 83, 255]);
        assert_eq!(&image.rgba[4..8], &[10, 10, 10, 255]);
        ink_rect(
            &mut image,
            SelectionRect {
                x: 2.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
        );
        assert_eq!(&image.rgba[8..12], &[INK_RGB.0, INK_RGB.1, INK_RGB.2, 255]);
    }
}
