use std::time::Instant;

use hayro::hayro_interpret::font::{Glyph as HayroGlyph, GlyphRun};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::hayro_syntax::Pdf;
use hayro::hayro_interpret::{
    interpret_page, BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image,
    ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask,
};
use hayro::kurbo::{Affine, BezPath, Point, Rect};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, PixmapSettings, RenderCache, RenderSettings};
use pdfium_render::prelude::*;
use spike_report::{rss_kb, PdfEngine, PdfSample, PixelScale, Quad};

const PHRASES: &[&str] = &["Few-Shot Learners", "GPT-3"];

/// Page 0 of the GPT-3 PDF puts footnote asterisks 3.6pt above the name baseline.
/// The next text line is 11pt below. A band of 5pt keeps the mark on its line.
const HAYRO_LINE_SLACK: f32 = 5.0;

/// On that page, a letter pair runs at most 0.5pt past the font advance.
/// A word space runs at least 2.2pt past it. The title word spaces are 4.3pt past it.
const HAYRO_WORD_EXTRA: f32 = 1.0;

#[derive(Clone)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct Glyph {
    pub index: u32,
    pub ch: char,
    pub quad: Quad,
    /// User-space pen advance. pdfium uses the tight box width. hayro uses the font advance.
    pub advance: f32,
}

pub struct PageText {
    pub plain: String,
    pub glyphs: Vec<Glyph>,
}

/// One link rectangle in PDF user space. `page` is 1-based when the link
/// stays in this file and names a page that exists. `None` is a URI, another
/// file, or a destination this file does not contain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageLink {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
    pub page: Option<u32>,
}

/// The topmost link under a PDF point. `Held` occupies the rectangle and
/// does not change the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkHit {
    Page(u32),
    Held,
}

pub enum Document {
    Pdfium(PdfiumDoc),
    Hayro(HayroDoc),
}

pub struct PdfiumDoc {
    pdfium: &'static Pdfium,
    doc: PdfDocument<'static>,
    count: usize,
}

pub struct HayroDoc {
    pdf: Pdf,
}

impl Document {
    pub fn open(
        engine: PdfEngine,
        bytes: Vec<u8>,
        lib_dir: &str,
    ) -> Result<(Self, f64, f64), String> {
        let started = Instant::now();
        match engine {
            PdfEngine::Pdfium => {
                let path = Pdfium::pdfium_platform_library_name_at_path(lib_dir);
                let pdfium = Box::leak(Box::new(match Pdfium::bind_to_library(&path) {
                    Ok(bindings) => Pdfium::new(bindings),
                    Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium::default(),
                    Err(err) => return Err(format!("bind pdfium: {err}")),
                }));
                let bind_ms = started.elapsed().as_secs_f64() * 1000.0;
                let opened = Instant::now();
                let doc = pdfium
                    .load_pdf_from_byte_vec(bytes, None)
                    .map_err(|e| format!("pdfium open: {e}"))?;
                let open_ms = opened.elapsed().as_secs_f64() * 1000.0;
                let count = doc.pages().len() as usize;
                Ok((
                    Document::Pdfium(PdfiumDoc { pdfium, doc, count }),
                    bind_ms,
                    open_ms,
                ))
            }
            PdfEngine::Hayro => {
                let bind_ms = 0.0;
                let opened = Instant::now();
                let pdf = Pdf::new(bytes).map_err(|e| format!("hayro open: {e:?}"))?;
                let open_ms = opened.elapsed().as_secs_f64() * 1000.0;
                Ok((Document::Hayro(HayroDoc { pdf }), bind_ms, open_ms))
            }
        }
    }

    pub fn page_count(&self) -> usize {
        match self {
            Document::Pdfium(doc) => doc.count,
            Document::Hayro(doc) => doc.pdf.pages().len(),
        }
    }

    pub fn page_size_pt(&self, index: usize) -> (f32, f32) {
        match self {
            Document::Pdfium(doc) => {
                let page = doc.doc.pages().get(index as i32).expect("page");
                (page.width().value, page.height().value)
            }
            Document::Hayro(doc) => doc.pdf.pages()[index].render_dimensions(),
        }
    }

    pub fn raster(&self, index: usize, scale: PixelScale) -> Result<Rgba, String> {
        match self {
            Document::Pdfium(doc) => raster_pdfium(doc, index, scale),
            Document::Hayro(doc) => raster_hayro(doc, index, scale),
        }
    }

    pub fn text(&self, index: usize) -> PageText {
        match self {
            Document::Pdfium(doc) => text_pdfium(doc, index),
            Document::Hayro(doc) => text_hayro(doc, index),
        }
    }

    /// Text for a search hit. Pdfium returns the real page text and tight
    /// quads. Hayro glyph boxes are not hit bounds. A failed read stays
    /// separate from a page that has no searchable characters.
    pub fn search_page(&self, index: usize) -> PageRead {
        match self {
            Document::Hayro(_) => PageRead::Failed("hayro".to_string()),
            Document::Pdfium(doc) => match read_pdfium(doc, index) {
                Ok(text) if text.plain.trim().is_empty() => PageRead::Empty,
                Ok(text) => PageRead::Text(text),
                Err(err) => PageRead::Failed(err),
            },
        }
    }

    /// Link rectangles on one page. Hayro does not read annotations.
    pub fn page_links(&self, index: usize) -> Vec<PageLink> {
        match self {
            Document::Pdfium(doc) => pdfium_page_links(doc, index),
            Document::Hayro(_) => Vec::new(),
        }
    }

    /// The document outline, depth-first. `page` is 1-based. Zero means the
    /// row has a title and no page in this file. Hayro does not read the tree.
    pub fn outline(&self) -> Vec<OutlineMark> {
        match self {
            Document::Pdfium(doc) => pdfium_outline(doc),
            Document::Hayro(_) => Vec::new(),
        }
    }
}

/// One outline row. `depth` is zero for a top-level bookmark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineMark {
    pub title: String,
    pub page: u32,
    pub depth: u32,
}

pub enum PageRead {
    Text(PageText),
    Empty,
    Failed(String),
}

fn pdfium_page_links(doc: &PdfiumDoc, index: usize) -> Vec<PageLink> {
    let Ok(page) = doc.doc.pages().get(index as i32) else {
        return Vec::new();
    };
    let (origin_x, origin_y) = rendered_origin(&page);
    let mut links = Vec::new();
    for link in page.links().iter() {
        let Ok(rect) = link.rect() else {
            continue;
        };
        links.push(PageLink {
            left: rect.left().value - origin_x,
            bottom: rect.bottom().value - origin_y,
            right: rect.right().value - origin_x,
            top: rect.top().value - origin_y,
            page: same_document_page(&link, doc.count),
        });
    }
    links
}

/// The bitmap's bottom-left is the crop box when the page has one, and the
/// media box otherwise. Link rectangles are stored in that painted space.
fn rendered_origin(page: &PdfPage<'_>) -> (f32, f32) {
    let bounds = page.boundaries();
    let rect = bounds.crop().or_else(|_| bounds.media()).ok();
    match rect {
        Some(boundary) => (boundary.bounds.left().value, boundary.bounds.bottom().value),
        None => (0.0, 0.0),
    }
}

fn pdfium_outline(doc: &PdfiumDoc) -> Vec<OutlineMark> {
    let Some(root) = doc.doc.bookmarks().root() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk_bookmark(root, 0, doc.count, &mut out);
    out
}

fn walk_bookmark(mark: PdfBookmark<'_>, depth: u32, pages: usize, out: &mut Vec<OutlineMark>) {
    let mut current = Some(mark);
    while let Some(mark) = current {
        let title = mark.title().unwrap_or_default();
        let title = title.trim();
        if !title.is_empty() {
            out.push(OutlineMark {
                title: title.to_string(),
                page: bookmark_page(&mark, pages),
                depth,
            });
        }
        if let Some(child) = mark.first_child() {
            walk_bookmark(child, depth.saturating_add(1), pages, out);
        }
        current = mark.next_sibling();
    }
}

fn bookmark_page(mark: &PdfBookmark<'_>, pages: usize) -> u32 {
    let index = mark
        .destination()
        .and_then(|dest| dest.page_index().ok())
        .or_else(|| match mark.action() {
            Some(PdfAction::LocalDestination(action)) => action
                .destination()
                .ok()
                .and_then(|dest| dest.page_index().ok()),
            _ => None,
        });
    let Some(index) = index else {
        return 0;
    };
    if index < 0 || index as usize >= pages {
        return 0;
    }
    index as u32 + 1
}

fn same_document_page(link: &PdfLink<'_>, pages: usize) -> Option<u32> {
    let index = match link.action() {
        Some(PdfAction::LocalDestination(action)) => action.destination().ok()?.page_index().ok(),
        Some(_) => return None,
        None => link.destination().and_then(|dest| dest.page_index().ok()),
    }?;
    if index < 0 || index as usize >= pages {
        return None;
    }
    Some(index as u32 + 1)
}

/// `y` grows down from `top`, matching the flipped page view.
/// The returned PDF y grows up from the bottom of the page.
pub fn pdf_user_point(
    x: f64,
    y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    page_w: f32,
    page_h: f32,
) -> Option<(f32, f32)> {
    if !(width > 0.0 && height > 0.0 && page_w > 0.0 && page_h > 0.0) {
        return None;
    }
    if x < left || y < top || x > left + width || y > top + height {
        return None;
    }
    let px = ((x - left) / width) * f64::from(page_w);
    let down = ((y - top) / height) * f64::from(page_h);
    Some((px as f32, (f64::from(page_h) - down) as f32))
}

/// A margin that covers the page owns this point. The page beside it does not.
pub fn margin_owns_point(
    x: f64,
    y: f64,
    covers: bool,
    margin: Option<(f64, f64, f64, f64)>,
) -> bool {
    if !covers {
        return false;
    }
    let Some((left, top, width, height)) = margin else {
        return false;
    };
    width > 0.0 && height > 0.0 && x >= left && y >= top && x <= left + width && y <= top + height
}

/// Map a view click through the painted page and test that page's links.
pub fn hit_fitted_page(
    x: f64,
    y: f64,
    paint_x: f64,
    paint_y: f64,
    paint_w: f64,
    paint_h: f64,
    page_w: f32,
    page_h: f32,
    links: &[PageLink],
    page: u32,
) -> Option<(u32, LinkHit)> {
    let (px, py) = pdf_user_point(x, y, paint_x, paint_y, paint_w, paint_h, page_w, page_h)?;
    let hit = hit_page_link(links, px, py)?;
    Some((page, hit))
}

/// A cross-page GoTo. The same page, a page this file does not contain,
/// and a page of 0 stay. The first move records `here`. A later move keeps it.
pub fn pdf_link_jump(
    here: u32,
    dest: u32,
    pages: u32,
    back: Option<u32>,
) -> Option<(u32, Option<u32>)> {
    if dest == 0 || pages == 0 || dest > pages || dest == here {
        return None;
    }
    let back = if back.is_none() { Some(here) } else { back };
    Some((dest, back))
}

/// Opening a book drops the return page. The same open book keeps it.
pub fn link_back_for_book(back: Option<u32>, same_book: bool) -> Option<u32> {
    if same_book {
        back
    } else {
        None
    }
}

/// The last rectangle that contains the point wins. Edges are inside.
pub fn hit_page_link(links: &[PageLink], x: f32, y: f32) -> Option<LinkHit> {
    let mut found = None;
    for link in links {
        let left = link.left.min(link.right);
        let right = link.left.max(link.right);
        let bottom = link.bottom.min(link.top);
        let top = link.bottom.max(link.top);
        if x >= left && x <= right && y >= bottom && y <= top {
            found = Some(match link.page {
                Some(page) => LinkHit::Page(page),
                None => LinkHit::Held,
            });
        }
    }
    found
}

/// Page 1 as a PNG. Hayro draws it, so the call does not need pdfium.
pub fn first_page_png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let (doc, _, _) = Document::open(PdfEngine::Hayro, bytes.to_vec(), "")?;
    if doc.page_count() == 0 {
        return Err("pdf has no pages".into());
    }
    let image = fit_cover(doc.raster(0, PixelScale::Retina1x)?, 180);
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|err| err.to_string())?;
        writer
            .write_image_data(&image.bytes)
            .map_err(|err| err.to_string())?;
    }
    Ok(out)
}

fn fit_cover(image: Rgba, max_edge: u32) -> Rgba {
    let long = image.width.max(image.height);
    if long <= max_edge || image.width == 0 || image.height == 0 {
        return image;
    }
    let width = ((u64::from(image.width) * u64::from(max_edge)) / u64::from(long)).max(1) as u32;
    let height = ((u64::from(image.height) * u64::from(max_edge)) / u64::from(long)).max(1) as u32;
    let mut bytes = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height {
        let src_y = (u64::from(y) * u64::from(image.height) / u64::from(height)) as u32;
        for x in 0..width {
            let src_x = (u64::from(x) * u64::from(image.width) / u64::from(width)) as u32;
            let from = ((src_y * image.width + src_x) * 4) as usize;
            let to = ((y * width + x) * 4) as usize;
            bytes[to..to + 4].copy_from_slice(&image.bytes[from..from + 4]);
        }
    }
    Rgba {
        width,
        height,
        bytes,
    }
}

fn raster_pdfium(doc: &PdfiumDoc, index: usize, scale: PixelScale) -> Result<Rgba, String> {
    let page = doc
        .doc
        .pages()
        .get(index as i32)
        .map_err(|e| format!("page: {e}"))?;
    let px = (page.width().value * scale.px_per_pt()).round() as i32;
    let bitmap = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(px))
        .map_err(|e| format!("render: {e}"))?;
    let bytes = bitmap.as_rgba_bytes();
    let width = bitmap.width() as u32;
    let height = bitmap.height() as u32;
    if bytes.len() != width as usize * height as usize * 4 {
        return Err(format!(
            "rgba length {} != {}x{}x4",
            bytes.len(),
            width,
            height
        ));
    }
    Ok(Rgba {
        width,
        height,
        bytes,
    })
}

fn raster_hayro(doc: &HayroDoc, index: usize, scale: PixelScale) -> Result<Rgba, String> {
    let page = &doc.pdf.pages()[index];
    let (pw, ph) = page.render_dimensions();
    let px_w = (pw * scale.px_per_pt()).round() as u32;
    let px_h = (ph * scale.px_per_pt()).round() as u32;
    if px_w > u16::MAX as u32 || px_h > u16::MAX as u32 {
        return Err(format!("hayro pixmap {px_w}x{px_h} exceeds u16"));
    }
    let cache = RenderCache::new();
    let pixmap = render(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &PixmapSettings {
            x_scale: scale.px_per_pt(),
            y_scale: scale.px_per_pt(),
            bg_color: WHITE,
        },
    );
    let width = pixmap.width() as u32;
    let height = pixmap.height() as u32;
    let bytes = pixmap.data_as_u8_slice().to_vec();
    if bytes.len() != width as usize * height as usize * 4 {
        return Err(format!(
            "hayro rgba length {} != {}x{}x4",
            bytes.len(),
            width,
            height
        ));
    }
    Ok(Rgba {
        width,
        height,
        bytes,
    })
}

fn text_pdfium(doc: &PdfiumDoc, index: usize) -> PageText {
    read_pdfium(doc, index).unwrap_or(PageText {
        plain: String::new(),
        glyphs: Vec::new(),
    })
}

fn read_pdfium(doc: &PdfiumDoc, index: usize) -> Result<PageText, String> {
    let page = doc
        .doc
        .pages()
        .get(index as i32)
        .map_err(|err| format!("pdfium page: {err}"))?;
    let height = page.height().value;
    let text = page.text().map_err(|err| format!("pdfium text: {err}"))?;
    let mut glyphs = Vec::new();
    let mut plain = String::new();
    for ch in text.chars().iter() {
        let Some(c) = ch.unicode_char() else {
            continue;
        };
        let Ok(bounds) = ch.tight_bounds() else {
            continue;
        };
        let index = plain.chars().count() as u32;
        plain.push(c);
        let top = height - bounds.top().value;
        glyphs.push(Glyph {
            index,
            ch: c,
            quad: Quad {
                x: bounds.left().value,
                y: top,
                w: bounds.width().value.max(0.0),
                h: (bounds.top().value - bounds.bottom().value).abs().max(0.0),
            },
            advance: bounds.width().value.max(0.0),
        });
    }
    Ok(PageText { plain, glyphs })
}

fn text_hayro(doc: &HayroDoc, index: usize) -> PageText {
    let page = &doc.pdf.pages()[index];
    let (w, h) = page.render_dimensions();
    let cache = InterpreterCache::new();
    let mut context = Context::new(
        Affine::IDENTITY,
        Rect::new(0.0, 0.0, w as f64, h as f64),
        &cache,
        doc.pdf.xref(),
        InterpreterSettings::default(),
    );
    let mut extractor = Extractor {
        height: h,
        glyphs: Vec::new(),
    };
    interpret_page(page, &mut context, &mut extractor);
    reading_order(extractor.glyphs)
}

/// hayro draws glyphs and leaves word spaces as positioning gaps.
/// Cluster a line so a superscript stays with its baseline, then insert a space
/// when the next glyph starts more than its advance past the previous one.
fn reading_order(mut glyphs: Vec<Glyph>) -> PageText {
    glyphs.sort_by(|a, b| {
        a.quad
            .y
            .total_cmp(&b.quad.y)
            .then(a.quad.x.total_cmp(&b.quad.x))
    });
    let mut lines: Vec<Vec<Glyph>> = Vec::new();
    for glyph in glyphs {
        let same_line = lines.last().is_some_and(|line| {
            let top = line.iter().map(|g| g.quad.y).fold(f32::MAX, f32::min);
            glyph.quad.y - top <= HAYRO_LINE_SLACK
        });
        if same_line {
            lines.last_mut().unwrap().push(glyph);
        } else {
            lines.push(vec![glyph]);
        }
    }

    let mut plain = String::new();
    let mut ordered = Vec::new();
    for line in &mut lines {
        line.sort_by(|a, b| a.quad.x.total_cmp(&b.quad.x));
        for i in 0..line.len() {
            if i > 0 {
                let extra = line[i].quad.x - line[i - 1].quad.x - line[i - 1].advance;
                if extra > HAYRO_WORD_EXTRA {
                    plain.push(' ');
                }
            }
            line[i].index = plain.chars().count() as u32;
            plain.push(line[i].ch);
            ordered.push(line[i].clone());
        }
        plain.push('\n');
    }
    PageText {
        plain,
        glyphs: ordered,
    }
}

struct Extractor {
    height: f32,
    glyphs: Vec<Glyph>,
}

impl Device<'_> for Extractor {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'_>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}
    fn draw_image(&mut self, _: Image<'_, '_>, _: ImageDrawProps<'_>) {}

    fn draw_glyph_run(&mut self, glyph_run: &GlyphRun<'_, '_>, props: DrawProps<'_>, _: &DrawMode) {
        for glyph in glyph_run.glyphs() {
            let Some(unicode) = glyph.as_unicode() else {
                continue;
            };
            let text = match unicode {
                BfString::Char(c) => c.to_string(),
                BfString::String(s) => s,
            };
            let flip =
                Affine::translate((0.0, self.height as f64)) * Affine::scale_non_uniform(1.0, -1.0);
            let transform = flip * props.transform * glyph.transform();
            let position = transform * Point::ZERO;
            let units = match &**glyph {
                HayroGlyph::Outline(outline) => outline.advance_width().unwrap_or(0.0),
                HayroGlyph::Type3(type3) => type3.advance_width().unwrap_or(0.0),
            };
            let advanced = transform * Point::new(units as f64, 0.0);
            let advance = (advanced.x - position.x) as f32;
            for ch in text.chars() {
                self.glyphs.push(Glyph {
                    index: 0,
                    ch,
                    quad: Quad {
                        x: position.x as f32,
                        y: position.y as f32,
                        w: 6.0,
                        h: 10.0,
                    },
                    advance,
                });
            }
        }
    }
}

pub fn select_drag(
    text: &PageText,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> Option<spike_report::Anchor> {
    let left = x0.min(x1);
    let right = x0.max(x1);
    let top = y0.min(y1);
    let bottom = y0.max(y1);
    let mut hit: Vec<&Glyph> = text
        .glyphs
        .iter()
        .filter(|g| {
            let cx = g.quad.x + g.quad.w * 0.5;
            let cy = g.quad.y + g.quad.h * 0.5;
            cx >= left && cx <= right && cy >= top && cy <= bottom
        })
        .collect();
    if hit.is_empty() {
        return None;
    }
    hit.sort_by_key(|g| g.index);
    let start = hit[0].index;
    let end = hit.last().unwrap().index + 1;
    let selected: String = text
        .plain
        .chars()
        .skip(start as usize)
        .take((end - start) as usize)
        .collect();
    if selected.trim().is_empty() {
        return None;
    }
    let quads = hit.iter().map(|g| g.quad).collect();
    Some(spike_report::Anchor::Pdf {
        page: 0,
        start,
        end,
        text: selected,
        quads,
    })
}

pub fn sample(
    engine: PdfEngine,
    path: &str,
    scale: PixelScale,
    process_started: Instant,
) -> Result<PdfSample, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let (doc, bind_ms, open_ms) = Document::open(engine, bytes, &spike_report::pdfium_lib())?;
    let pages = doc.page_count();
    if pages < 50 {
        return Err(format!("{path} has {pages} pages, want at least 50"));
    }
    let mut errors = 0u32;
    let raster_started = Instant::now();
    let first = match doc.raster(0, scale) {
        Ok(image) => image,
        Err(e) => return Err(e),
    };
    let first_raster_ms = raster_started.elapsed().as_secs_f64() * 1000.0;
    let open_to_first_ms = process_started.elapsed().as_secs_f64() * 1000.0;
    let rgba_ok = first.bytes.len() == first.width as usize * first.height as usize * 4;
    if !rgba_ok {
        errors += 1;
    }
    let rss_after_first_kb = rss_kb();
    let mut later_page_ms = Vec::new();
    for index in 1..pages {
        let started = Instant::now();
        match doc.raster(index, scale) {
            Ok(image) => {
                let expect = image.width as usize * image.height as usize * 4;
                if image.bytes.len() != expect {
                    if errors == 0 {
                        eprintln!(
                            "page {index} rgba {} != {expect} ({}x{})",
                            image.bytes.len(),
                            image.width,
                            image.height
                        );
                    }
                    errors += 1;
                }
            }
            Err(err) => {
                if errors == 0 {
                    eprintln!("page {index} {err}");
                }
                errors += 1;
            }
        }
        later_page_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let rss_after_last_kb = rss_kb();
    let text = doc.text(0);
    let text_hits = PHRASES
        .iter()
        .filter(|phrase| text.plain.contains(**phrase))
        .count() as u32;
    Ok(PdfSample {
        engine: engine.name().into(),
        scale: scale.name().into(),
        release: !cfg!(debug_assertions),
        pages,
        bind_ms,
        open_ms,
        first_raster_ms,
        open_to_first_ms,
        later_page_ms,
        errors,
        rgba_ok,
        text_hits,
        text_chars: text.plain.chars().count(),
        glyph_boxes: text.glyphs.len(),
        rss_after_first_kb,
        rss_after_last_kb,
        px_w: first.width,
        px_h: first.height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hayro_search_does_not_return_glyph_boxes() {
        let bytes = b"%PDF-1.1\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF".to_vec();
        let (doc, _, _) =
            Document::open(PdfEngine::Hayro, bytes, &spike_report::pdfium_lib()).unwrap();
        assert!(matches!(doc.search_page(0), PageRead::Failed(_)));
        assert!(matches!(doc, Document::Hayro(_)));
    }

    #[test]
    fn a_find_mark_is_lighter_than_the_annotation_and_an_underline_does_not_fill() {
        let scale = PixelScale::Retina1x;
        let quad = Quad {
            x: 0.0,
            y: 0.0,
            w: 2.0,
            h: 2.0,
        };
        let white = Rgba {
            width: 4,
            height: 4,
            bytes: vec![255; 64],
        };
        let mut note = white.clone();
        let mut find = white.clone();
        paint_highlight(&mut note, &[quad], scale);
        paint_find(
            &mut find,
            &[FindMark {
                quad,
                current: false,
                underline: false,
            }],
            scale,
        );
        let note_ink = note.bytes[0] as u16 + note.bytes[1] as u16 + note.bytes[2] as u16;
        let find_ink = find.bytes[0] as u16 + find.bytes[1] as u16 + find.bytes[2] as u16;
        assert!(find_ink > note_ink);
        let mut line = white;
        paint_find(
            &mut line,
            &[FindMark {
                quad,
                current: false,
                underline: true,
            }],
            scale,
        );
        assert_eq!(&line.bytes[0..3], &[255, 255, 255]);
        assert_eq!(&line.bytes[48..51], &[184, 122, 40]);
    }

    fn four_words(line: &str) -> Option<String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let phrase = words.windows(4).find(|window| {
            window.iter().all(|word| {
                word.len() >= 3
                    && word
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '\'')
                    && !word.contains("fi")
                    && !word.contains("fl")
                    && !word.contains("ff")
            })
        })?;
        Some(phrase.join(" "))
    }

    #[test]
    fn select_points_keeps_glyphs_on_both_lines() {
        let glyph = |index, ch, x, y| Glyph {
            index,
            ch,
            quad: Quad {
                x,
                y,
                w: 8.0,
                h: 10.0,
            },
            advance: 8.0,
        };
        let text = PageText {
            plain: "Recne".into(),
            glyphs: vec![
                glyph(0, 'R', 10.0, 10.0),
                glyph(1, 'e', 20.0, 10.0),
                glyph(2, 'c', 30.0, 10.0),
                glyph(3, 'n', 10.0, 30.0),
                glyph(4, 'e', 20.0, 30.0),
            ],
        };
        let Some(spike_report::Anchor::Pdf {
            start,
            end,
            text,
            quads,
            ..
        }) = select_points(&text, (14.0, 15.0), (14.0, 35.0))
        else {
            panic!("two lines");
        };
        assert_eq!(start, 0);
        assert_eq!(end, 4);
        assert_eq!(text, "Recn");
        assert_eq!(quads.len(), 4);
        assert!((quads[0].y - 10.0).abs() < 0.01);
        assert!((quads[3].y - 30.0).abs() < 0.01);
    }

    #[test]
    fn hayro_word_gap_recovers_the_title_phrase() {
        let path = spike_report::gpt3_path();
        if !std::path::Path::new(&path).exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let (pdfium, _, _) = Document::open(
            PdfEngine::Pdfium,
            bytes.clone(),
            &spike_report::pdfium_lib(),
        )
        .unwrap();
        let (hayro, _, _) =
            Document::open(PdfEngine::Hayro, bytes, &spike_report::pdfium_lib()).unwrap();
        let pdfium_page = pdfium.text(0).plain;
        let hayro_page = hayro.text(0).plain;
        assert!(pdfium_page.contains("Few-Shot Learners"));
        assert!(
            hayro_page.contains("Few-Shot Learners"),
            "hayro title after the gap pass: {}",
            hayro_page.lines().next().unwrap_or("")
        );
        assert!(hayro_page.contains("GPT-3"));
        assert!(
            hayro_page.contains("Brown∗"),
            "footnote mark should stay on the name line"
        );
        assert!(
            hayro_page.contains('ﬁ'),
            "hayro still emits the fi ligature"
        );
        assert!(pdfium_page.contains("fi") && !pdfium_page.contains('ﬁ'));
        assert!(hayro_page.contains('-') && pdfium_page.contains('-'));
        assert_eq!(pdfium_page.chars().filter(|ch| *ch == '\u{2}').count(), 1);
        assert!(!hayro_page.contains('\u{2}'));

        for index in [1usize, 2, 10] {
            let page = pdfium.text(index).plain;
            let from_pdfium = page
                .lines()
                .find_map(four_words)
                .unwrap_or_else(|| panic!("page {index} has no four-word line"));
            let from_hayro = hayro.text(index).plain;
            assert!(
                from_hayro.contains(&from_pdfium),
                "page {index} missing {from_pdfium:?}"
            );
        }
    }

    #[test]
    fn the_shelf_field_guide_is_not_a_blank_page() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../library/fixtures/apple/Field Guide.pdf"
        );
        let bytes = std::fs::read(path).unwrap();
        let (doc, _, _) =
            Document::open(PdfEngine::Hayro, bytes, &spike_report::pdfium_lib()).unwrap();
        let image = doc.raster(0, PixelScale::Retina1x).unwrap();
        let dark = image
            .bytes
            .chunks(4)
            .filter(|px| px.len() == 4 && px[0] < 40 && px[1] < 40 && px[2] < 40)
            .count();
        assert!(dark > 100, "dark pixels {dark}");
    }

    #[test]
    fn highlight_keeps_the_ink_and_sits_on_abstract() {
        let path = spike_report::gpt3_path();
        if !std::path::Path::new(&path).exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let (doc, _, _) =
            Document::open(PdfEngine::Pdfium, bytes, &spike_report::pdfium_lib()).unwrap();
        let scale = PixelScale::Retina1x;
        let mut image = doc.raster(0, scale).unwrap();
        let text = doc.text(0);
        let start = text.plain.find("Abstract").expect("Abstract");
        let from = text.plain[..start].chars().count() as u32;
        let quads: Vec<Quad> = text
            .glyphs
            .iter()
            .filter(|glyph| glyph.index >= from && glyph.index < from + 8)
            .map(|glyph| glyph.quad)
            .collect();
        assert_eq!(quads.len(), 8, "one box per letter of Abstract");
        let before = ink_in(&image, &quads, scale);
        paint_highlight(&mut image, &quads, scale);
        let after = ink_in(&image, &quads, scale);
        assert!(
            before > 20,
            "the word's ink is inside the quads, before={before}"
        );
        assert!(
            after + 5 >= before,
            "ink survived the tint, before={before} after={after}"
        );
    }

    #[test]
    fn a_goto_link_keeps_its_rect_and_page() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/goto-links.pdf");
        let bytes = std::fs::read(path).unwrap();
        let (doc, _, _) = Document::open(
            PdfEngine::Pdfium,
            bytes.clone(),
            &spike_report::pdfium_lib(),
        )
        .unwrap();
        assert_eq!(doc.page_count(), 2);
        let (width, height) = doc.page_size_pt(0);
        assert!((width - 200.0).abs() < 0.2, "{width}");
        assert!((height - 300.0).abs() < 0.2, "{height}");
        let links = doc.page_links(0);
        let goto = links
            .iter()
            .find(|link| link.page == Some(2))
            .expect("page 1 GoTo");
        assert!((goto.left - 20.0).abs() < 0.2);
        assert!((goto.bottom - 260.0).abs() < 0.2);
        assert!((goto.right - 90.0).abs() < 0.2);
        assert!((goto.top - 290.0).abs() < 0.2);
        let uri = links
            .iter()
            .find(|link| (link.left - 110.0).abs() < 0.2 && link.bottom > 200.0)
            .expect("uri");
        assert_eq!(uri.page, None);
        let missing = links
            .iter()
            .find(|link| (link.left - 20.0).abs() < 0.2 && link.bottom < 40.0)
            .expect("missing dest");
        assert_eq!(missing.page, None);
        let other = links
            .iter()
            .find(|link| (link.left - 110.0).abs() < 0.2 && link.bottom < 40.0)
            .expect("other file");
        assert_eq!(other.page, None);
        let same = links
            .iter()
            .find(|link| link.page == Some(1))
            .expect("same page");
        assert!((same.bottom - 140.0).abs() < 0.2);
        let back = doc.page_links(1);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].page, Some(1));
        assert!((back[0].left - 20.0).abs() < 0.2 && (back[0].top - 290.0).abs() < 0.2);
        assert_eq!(
            hit_page_link(&links, goto.right, goto.top),
            Some(LinkHit::Page(2))
        );
        assert_eq!(hit_page_link(&links, goto.right + 0.2, goto.top), None);
        assert_eq!(
            hit_page_link(&links, uri.left, uri.bottom),
            Some(LinkHit::Held)
        );
        let stacked = [
            PageLink {
                left: 0.0,
                bottom: 0.0,
                right: 10.0,
                top: 10.0,
                page: Some(2),
            },
            PageLink {
                left: 0.0,
                bottom: 0.0,
                right: 10.0,
                top: 10.0,
                page: None,
            },
        ];
        assert_eq!(hit_page_link(&stacked, 5.0, 5.0), Some(LinkHit::Held));
        let view = |width: f64, height: f64, px: f32, py: f32| {
            let x = 100.0 + f64::from(px) / 200.0 * width;
            let y = 40.0 + (300.0 - f64::from(py)) / 300.0 * height;
            (x, y)
        };
        let (x, y) = view(100.0, 150.0, goto.right, goto.top);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0, &links, 1),
            Some((1, LinkHit::Page(2)))
        );
        let (x, y) = view(400.0, 600.0, 55.0, 275.0);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 400.0, 600.0, 200.0, 300.0, &links, 1)
                .map(|(_, hit)| hit),
            Some(LinkHit::Page(2))
        );
        assert_eq!(
            pdf_user_point(99.9, 40.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0),
            None
        );
        let top = pdf_user_point(200.0, 40.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0).unwrap();
        assert!((top.0 - 200.0).abs() < 0.01 && (top.1 - 300.0).abs() < 0.01);
        let bottom = pdf_user_point(100.0, 190.0, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0).unwrap();
        assert!(bottom.0.abs() < 0.01 && bottom.1.abs() < 0.01);
        assert!(margin_owns_point(
            10.0,
            10.0,
            true,
            Some((0.0, 0.0, 40.0, 40.0))
        ));
        assert!(!margin_owns_point(
            10.0,
            10.0,
            false,
            Some((0.0, 0.0, 40.0, 40.0))
        ));
        assert_eq!(pdf_link_jump(1, 2, 2, None), Some((2, Some(1))));
        assert_eq!(pdf_link_jump(2, 1, 2, Some(1)), Some((1, Some(1))));
        assert_eq!(pdf_link_jump(1, 2, 2, Some(1)), Some((2, Some(1))));
        assert_eq!(pdf_link_jump(1, 1, 2, None), None);
        assert_eq!(pdf_link_jump(1, 0, 2, None), None);
        assert_eq!(pdf_link_jump(1, 9, 2, None), None);
        assert_eq!(link_back_for_book(Some(1), true), Some(1));
        assert_eq!(link_back_for_book(Some(1), false), None);
        let (hayro, _, _) =
            Document::open(PdfEngine::Hayro, bytes, &spike_report::pdfium_lib()).unwrap();
        assert!(hayro.page_links(0).is_empty());
    }

    #[test]
    fn a_shifted_page_box_hits_the_painted_ink() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/shifted-box.pdf");
        let bytes = std::fs::read(path).unwrap();
        let (doc, _, _) =
            Document::open(PdfEngine::Pdfium, bytes, &spike_report::pdfium_lib()).unwrap();
        let (width, height) = doc.page_size_pt(0);
        assert!((width - 200.0).abs() < 0.2 && (height - 300.0).abs() < 0.2);
        let links = doc.page_links(0);
        assert_eq!(links.len(), 1);
        let link = links[0];
        assert_eq!(link.page, Some(2));
        assert!((link.left - 20.0).abs() < 0.2 && (link.bottom - 20.0).abs() < 0.2);
        assert!((link.right - 90.0).abs() < 0.2 && (link.top - 50.0).abs() < 0.2);
        let image = doc.raster(0, PixelScale::Retina1x).unwrap();
        let dark = dark_bbox(&image);
        assert_eq!(dark, (40, 500, 179, 559), "ink bbox {dark:?}");
        let (x, y) = painted_click(55.0, 35.0, 200.0, 300.0, 100.0, 150.0);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0, &links, 1),
            Some((1, LinkHit::Page(2)))
        );
        let (miss_x, miss_y) = painted_click(55.0, 135.0, 200.0, 300.0, 100.0, 150.0);
        assert_eq!(
            hit_fitted_page(miss_x, miss_y, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0, &links, 1),
            None
        );
        let cropped = doc.page_links(1);
        assert_eq!(cropped.len(), 1);
        assert_eq!(cropped[0].page, Some(1));
        assert!((cropped[0].left - 20.0).abs() < 0.2 && (cropped[0].bottom - 260.0).abs() < 0.2);
        assert!((cropped[0].right - 90.0).abs() < 0.2 && (cropped[0].top - 290.0).abs() < 0.2);
        let crop = doc.raster(1, PixelScale::Retina1x).unwrap();
        assert_eq!(dark_bbox(&crop), (40, 20, 179, 79));
        let (x, y) = painted_click(55.0, 275.0, 200.0, 300.0, 100.0, 150.0);
        assert_eq!(
            hit_fitted_page(x, y, 100.0, 40.0, 100.0, 150.0, 200.0, 300.0, &cropped, 2)
                .map(|(_, hit)| hit),
            Some(LinkHit::Page(1))
        );
    }

    fn painted_click(
        px: f32,
        py: f32,
        page_w: f32,
        page_h: f32,
        width: f64,
        height: f64,
    ) -> (f64, f64) {
        let x = 100.0 + f64::from(px) / f64::from(page_w) * width;
        let y = 40.0 + (f64::from(page_h) - f64::from(py)) / f64::from(page_h) * height;
        (x, y)
    }

    fn dark_bbox(image: &Rgba) -> (i32, i32, i32, i32) {
        let mut left = image.width as i32;
        let mut top = image.height as i32;
        let mut right = 0i32;
        let mut bottom = 0i32;
        for y in 0..image.height as i32 {
            for x in 0..image.width as i32 {
                let i = ((y as u32 * image.width + x as u32) * 4) as usize;
                if image.bytes[i] < 40 && image.bytes[i + 1] < 40 && image.bytes[i + 2] < 40 {
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x);
                    bottom = bottom.max(y);
                }
            }
        }
        (left, top, right, bottom)
    }

    fn ink_in(image: &Rgba, quads: &[Quad], scale: PixelScale) -> usize {
        let mut n = 0;
        for quad in quads {
            let px = quad.pixels(scale);
            let x0 = px.x.round().max(0.0) as i32;
            let y0 = px.y.round().max(0.0) as i32;
            let x1 = (px.x + px.w).round() as i32;
            let y1 = (px.y + px.h).round() as i32;
            for y in y0..y1 {
                for x in x0..x1 {
                    if y < 0 || x < 0 || y >= image.height as i32 || x >= image.width as i32 {
                        continue;
                    }
                    let i = ((y as u32 * image.width + x as u32) * 4) as usize;
                    if image.bytes[i] < 96 && image.bytes[i + 1] < 96 && image.bytes[i + 2] < 96 {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    #[test]
    fn an_outline_lists_each_destination_page() {
        let bytes = outline_fixture();
        let (doc, _, _) = Document::open(
            PdfEngine::Pdfium,
            bytes.clone(),
            &spike_report::pdfium_lib(),
        )
        .expect("pdfium outline");
        let marks = doc.outline();
        assert_eq!(marks.len(), 3, "{marks:?}");
        assert_eq!(marks[0].title, "One");
        assert_eq!(marks[0].page, 1);
        assert_eq!(marks[0].depth, 0);
        assert_eq!(marks[1].title, "Nested");
        assert_eq!(marks[1].page, 2);
        assert_eq!(marks[1].depth, 1);
        assert_eq!(marks[2].title, "Two");
        assert_eq!(marks[2].page, 2);
        assert_eq!(marks[2].depth, 0);
        let (hayro, _, _) =
            Document::open(PdfEngine::Hayro, bytes, &spike_report::pdfium_lib()).unwrap();
        assert!(hayro.outline().is_empty());
    }

    /// Two pages. The outline is One, its child Nested, then Two.
    fn outline_fixture() -> Vec<u8> {
        let objects = [
            b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>\nendobj\n".to_vec(),
            b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>\nendobj\n".to_vec(),
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>\nendobj\n".to_vec(),
            b"4 0 obj\n<< /Type /Outlines /First 6 0 R /Last 7 0 R /Count 2 >>\nendobj\n".to_vec(),
            b"5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] >>\nendobj\n".to_vec(),
            b"6 0 obj\n<< /Title (One) /Parent 4 0 R /Dest [3 0 R /Fit] /Next 7 0 R /First 8 0 R /Last 8 0 R /Count 1 >>\nendobj\n".to_vec(),
            b"7 0 obj\n<< /Title (Two) /Parent 4 0 R /Dest [5 0 R /Fit] /Prev 6 0 R >>\nendobj\n".to_vec(),
            b"8 0 obj\n<< /Title (Nested) /Parent 6 0 R /Dest [5 0 R /Fit] >>\nendobj\n".to_vec(),
        ];
        let mut body = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for object in objects {
            offsets.push(body.len());
            body.extend_from_slice(&object);
        }
        let xref = body.len();
        let count = offsets.len() + 1;
        let mut table = format!("xref\n0 {count}\n0000000000 65535 f \n");
        for offset in offsets {
            table.push_str(&format!("{offset:010} 00000 n \n"));
        }
        body.extend(table.as_bytes());
        let trailer =
            format!("trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n");
        body.extend(trailer.as_bytes());
        body
    }
}

pub struct FindMark {
    pub quad: Quad,
    pub current: bool,
    pub underline: bool,
}

/// A search mark. Lighter than the annotation yellow. The current hit is
/// deeper and keeps a thin edge. An underline is the overlap: the annotation
/// fill stays, and this mark does not cover it.
pub fn paint_find(image: &mut Rgba, marks: &[FindMark], scale: PixelScale) {
    for mark in marks {
        if mark.underline {
            paint_find_line(image, &mark.quad, scale);
            continue;
        }
        let (red, green, blue, mix) = if mark.current {
            (232, 160, 48, 5)
        } else {
            (255, 236, 190, 4)
        };
        paint_find_fill(image, &mark.quad, scale, red, green, blue, mix);
        if mark.current {
            paint_find_line(image, &mark.quad, scale);
        }
    }
}

fn paint_find_fill(
    image: &mut Rgba,
    quad: &Quad,
    scale: PixelScale,
    red: u16,
    green: u16,
    blue: u16,
    mix: u16,
) {
    let px = quad.pixels(scale);
    let x0 = px.x.round().max(0.0) as i32;
    let y0 = px.y.round().max(0.0) as i32;
    let x1 = (px.x + px.w).round() as i32;
    let y1 = (px.y + px.h).round() as i32;
    let keep = 10 - mix;
    for y in y0..y1 {
        if y < 0 || y >= image.height as i32 {
            continue;
        }
        for x in x0..x1 {
            if x < 0 || x >= image.width as i32 {
                continue;
            }
            let i = ((y as u32 * image.width + x as u32) * 4) as usize;
            let lum = image.bytes[i] as u16 + image.bytes[i + 1] as u16 + image.bytes[i + 2] as u16;
            if lum < 540 {
                continue;
            }
            image.bytes[i] = ((image.bytes[i] as u16 * keep + red * mix) / 10) as u8;
            image.bytes[i + 1] = ((image.bytes[i + 1] as u16 * keep + green * mix) / 10) as u8;
            image.bytes[i + 2] = ((image.bytes[i + 2] as u16 * keep + blue * mix) / 10) as u8;
            image.bytes[i + 3] = 255;
        }
    }
}

fn paint_find_line(image: &mut Rgba, quad: &Quad, scale: PixelScale) {
    let px = quad.pixels(scale);
    let x0 = px.x.round().max(0.0) as i32;
    let x1 = (px.x + px.w).round() as i32;
    let y = (px.y + px.h).round() as i32 - 1;
    if y < 0 || y >= image.height as i32 {
        return;
    }
    for x in x0..x1 {
        if x < 0 || x >= image.width as i32 {
            continue;
        }
        let i = ((y as u32 * image.width + x as u32) * 4) as usize;
        image.bytes[i] = 184;
        image.bytes[i + 1] = 122;
        image.bytes[i + 2] = 40;
        image.bytes[i + 3] = 255;
    }
}

pub fn paint_highlight(image: &mut Rgba, quads: &[Quad], scale: PixelScale) {
    for quad in quads {
        let mut band = *quad;
        band.y = (band.y - 1.5).max(0.0);
        band.h += 3.0;
        let px = band.pixels(scale);
        let x0 = px.x.round().max(0.0) as i32;
        let y0 = px.y.round().max(0.0) as i32;
        let x1 = (px.x + px.w).round() as i32;
        let y1 = (px.y + px.h).round() as i32;
        for y in y0..y1 {
            if y < 0 || y >= image.height as i32 {
                continue;
            }
            for x in x0..x1 {
                if x < 0 || x >= image.width as i32 {
                    continue;
                }
                let i = ((y as u32 * image.width + x as u32) * 4) as usize;
                tint_under_ink(&mut image.bytes[i..i + 4]);
            }
        }
    }
}

fn tint_under_ink(px: &mut [u8]) {
    let lum = px[0] as u16 + px[1] as u16 + px[2] as u16;
    if lum < 540 {
        px[3] = 255;
        return;
    }
    px[0] = ((px[0] as u16 * 2 + 255 * 8) / 10) as u8;
    px[1] = ((px[1] as u16 * 2 + 210 * 8) / 10) as u8;
    px[2] = ((px[2] as u16 * 2 + 40 * 8) / 10) as u8;
    px[3] = 255;
}

pub fn select_points(
    text: &PageText,
    a: (f32, f32),
    b: (f32, f32),
) -> Option<spike_report::Anchor> {
    let start = nearest(text, a.0, a.1)?;
    let end = nearest(text, b.0, b.1)?;
    let (from, to) = if start.index <= end.index {
        (start.index, end.index)
    } else {
        (end.index, start.index)
    };
    let glyphs: Vec<&Glyph> = text
        .glyphs
        .iter()
        .filter(|glyph| glyph.index >= from && glyph.index <= to)
        .collect();
    if glyphs.is_empty() {
        return None;
    }
    let selected: String = text
        .plain
        .chars()
        .skip(from as usize)
        .take((to - from + 1) as usize)
        .collect();
    if selected.trim().is_empty() {
        return None;
    }
    Some(spike_report::Anchor::Pdf {
        page: 0,
        start: from,
        end: to + 1,
        text: selected,
        quads: glyphs.iter().map(|glyph| glyph.quad).collect(),
    })
}

fn nearest(text: &PageText, x: f32, y: f32) -> Option<&Glyph> {
    text.glyphs.iter().min_by(|p, q| {
        let dp = (p.quad.x + p.quad.w * 0.5 - x).hypot(p.quad.y + p.quad.h * 0.5 - y);
        let dq = (q.quad.x + q.quad.w * 0.5 - x).hypot(q.quad.y + q.quad.h * 0.5 - y);
        dp.total_cmp(&dq)
    })
}
