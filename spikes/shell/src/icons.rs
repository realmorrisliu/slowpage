use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2::AnyThread;
use objc2_app_kit::NSImage;
use objc2_foundation::{NSData, NSSize};

const LIST: &str = include_str!("../../../assets/icons/list.svg");
const SETTINGS: &str = include_str!("../../../assets/icons/settings.svg");
const SETTINGS_SOLID: &str = include_str!("../../../assets/icons/settings-solid.svg");
const PLUS: &str = include_str!("../../../assets/icons/plus.svg");
const PLUS_SOLID: &str = include_str!("../../../assets/icons/plus-solid.svg");
const FILE: &str = include_str!("../../../assets/icons/file.svg");
const FILE_SOLID: &str = include_str!("../../../assets/icons/file-solid.svg");
const CHECK: &str = include_str!("../../../assets/icons/check.svg");
const CHECK_SOLID: &str = include_str!("../../../assets/icons/check-solid.svg");
const BOOK_OPEN: &str = include_str!("../../../assets/icons/book-open.svg");
const BOOK_OPEN_SOLID: &str = include_str!("../../../assets/icons/book-open-solid.svg");
const SIDEBAR_RIGHT: &str = include_str!("../../../assets/icons/sidebar-right.svg");
const LIBRARY_SOLID: &str = include_str!("../../../assets/icons/library-solid.svg");
const BOOKMARK_SOLID: &str = include_str!("../../../assets/icons/bookmark-solid.svg");
const NOTE_SOLID: &str = include_str!("../../../assets/icons/note-solid.svg");
const FOLDER_SOLID: &str = include_str!("../../../assets/icons/folder-solid.svg");
const BOOKMARK: &str = include_str!("../../../assets/icons/bookmark.svg");
const SEARCH: &str = include_str!("../../../assets/icons/search.svg");
const BOOKMARK_PLUS: &str = include_str!("../../../assets/icons/bookmark-plus.svg");
const NOTE: &str = include_str!("../../../assets/icons/note.svg");
const BOOK: &str = include_str!("../../../assets/icons/book.svg");
const FOLDER: &str = include_str!("../../../assets/icons/folder.svg");
const LIBRARY: &str = include_str!("../../../assets/icons/library.svg");
const SIDEBAR_LEFT: &str = include_str!("../../../assets/icons/sidebar-left.svg");
const SLIDERS: &str = include_str!("../../../assets/icons/sliders.svg");
const SLIDERS_SOLID: &str = include_str!("../../../assets/icons/sliders-solid.svg");
const SORT_DESC: &str = include_str!("../../../assets/icons/sort-desc.svg");
const CHEVRON_DOWN: &str = include_str!("../../../assets/icons/chevron-down.svg");
const GRID: &str = include_str!("../../../assets/icons/grid.svg");
const EDIT: &str = include_str!("../../../assets/icons/edit.svg");
const PAPERCLIP: &str = include_str!("../../../assets/icons/paperclip.svg");
const ARROW_UP: &str = include_str!("../../../assets/icons/arrow-up.svg");
const MAXIMIZE: &str = include_str!("../../../assets/icons/maximize.svg");
const POINTS: f64 = 18.0;
const PIXELS: u32 = 54;

pub fn contents_image() -> Option<Retained<NSImage>> {
    image(LIST)
}

pub fn bookmark_image() -> Option<Retained<NSImage>> {
    image(BOOKMARK)
}

pub fn search_image() -> Option<Retained<NSImage>> {
    image(SEARCH)
}

pub fn add_bookmark_image() -> Option<Retained<NSImage>> {
    image(BOOKMARK_PLUS)
}

pub fn notes_image() -> Option<Retained<NSImage>> {
    image(NOTE)
}

pub fn glyph_source(name: &str) -> Option<&'static str> {
    named_svg(name)
}

pub fn glyph_file(name: &str) -> Option<String> {
    glyph_ink(name, "000000")
}

/// A `data:` URL for the painted glyph. The bytes stay in this process.
/// Nothing is written under Application Support, including when
/// `SUPERREADER_SUPPORT_DIR` is unset.
pub fn glyph_ink(name: &str, hex: &str) -> Option<String> {
    let key = format!("{name}-{hex}");
    if let Some(hit) = glyph_memory()
        .lock()
        .ok()
        .and_then(|map| map.get(&key).cloned())
    {
        return Some(hit);
    }
    let svg = named_svg(name)?;
    let png = raster_png(&paint_hex(svg, hex), 48).ok()?;
    let src = format!("data:image/png;base64,{}", base64(&png));
    if let Ok(mut map) = glyph_memory().lock() {
        map.insert(key, src.clone());
    }
    Some(src)
}

fn glyph_memory() -> &'static Mutex<HashMap<String, String>> {
    static MEMORY: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    MEMORY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let bits = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((bits >> 18) & 63) as usize] as char);
        out.push(TABLE[((bits >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((bits >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(bits & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn named_svg(name: &str) -> Option<&'static str> {
    Some(match name {
        "list" => LIST,
        "search" => SEARCH,
        "bookmark" => BOOKMARK,
        "bookmark-plus" => BOOKMARK_PLUS,
        "note" => NOTE,
        "book" => BOOK,
        "folder" => FOLDER,
        "library" => LIBRARY,
        "settings" => SETTINGS,
        "settings-solid" => SETTINGS_SOLID,
        "plus" => PLUS,
        "plus-solid" => PLUS_SOLID,
        "file" => FILE,
        "file-solid" => FILE_SOLID,
        "check" => CHECK,
        "check-solid" => CHECK_SOLID,
        "book-open" => BOOK_OPEN,
        "book-open-solid" => BOOK_OPEN_SOLID,
        "sidebar-right" => SIDEBAR_RIGHT,
        "library-solid" => LIBRARY_SOLID,
        "bookmark-solid" => BOOKMARK_SOLID,
        "note-solid" => NOTE_SOLID,
        "folder-solid" => FOLDER_SOLID,
        "sidebar-left" => SIDEBAR_LEFT,
        "sliders" => SLIDERS,
        "sliders-solid" => SLIDERS_SOLID,
        "sort-desc" => SORT_DESC,
        "chevron-down" => CHEVRON_DOWN,
        "grid" => GRID,
        "edit" => EDIT,
        "paperclip" => PAPERCLIP,
        "arrow-up" => ARROW_UP,
        "maximize" => MAXIMIZE,
        _ => return None,
    })
}

fn image(svg: &str) -> Option<Retained<NSImage>> {
    let png = raster_png(svg, PIXELS).ok()?;
    let data = NSData::with_bytes(&png);
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    image.setSize(NSSize::new(POINTS, POINTS));
    image.setTemplate(true);
    Some(image)
}

fn raster_png(svg: &str, px: u32) -> Result<Vec<u8>, String> {
    let pixmap = raster(svg, px)?;
    pixmap.encode_png().map_err(|err| err.to_string())
}

fn raster(svg: &str, px: u32) -> Result<resvg::tiny_skia::Pixmap, String> {
    let painted = paint(svg);
    let tree = resvg::usvg::Tree::from_str(&painted, &resvg::usvg::Options::default())
        .map_err(|err| err.to_string())?;
    let pixmap = draw(&tree, px)?;
    let ink = pixmap.pixels().iter().filter(|px| px.alpha() > 0).count();
    if ink == 0 {
        return Err("icon raster is empty".into());
    }
    Ok(pixmap)
}

fn paint(svg: &str) -> String {
    paint_hex(svg, "000000")
}

fn paint_hex(svg: &str, hex: &str) -> String {
    // usvg leaves currentColor unresolved, which drops the stroke.
    let fill = format!("#{hex}");
    svg.replace("currentColor", &fill)
}

fn draw(tree: &resvg::usvg::Tree, px: u32) -> Result<resvg::tiny_skia::Pixmap, String> {
    let size = tree.size();
    let width = size.width();
    let height = size.height();
    if width <= 0.0 || height <= 0.0 {
        return Err("svg has no size".into());
    }
    let scale = px as f32 / width;
    let w = (width * scale).ceil().max(1.0) as u32;
    let h = (height * scale).ceil().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("pixmap")?;
    resvg::render(
        tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_toolbar_icon_rasterizes_to_ink() {
        for svg in [
            LIST,
            BOOKMARK,
            SEARCH,
            BOOKMARK_PLUS,
            NOTE,
            BOOK,
            FOLDER,
            LIBRARY,
        ] {
            let pixmap = raster(svg, 32).unwrap();
            let ink = pixmap.pixels().iter().filter(|px| px.alpha() > 0).count();
            assert!(ink > 20, "ink pixels {ink}");
        }
    }

    #[test]
    fn glyph_ink_writes_nothing_when_the_support_dir_is_overridden() {
        let dir =
            std::env::temp_dir().join(format!("superreader-glyph-memory-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let previous = std::env::var("SUPERREADER_SUPPORT_DIR").ok();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        let real = crate::settings::support_dir();
        assert_eq!(real, dir);
        let home = crate::settings::default_support_dir().join("shell-glyphs");
        let before = names_in(&home);
        let src = glyph_ink("search", "1D1D1F").expect("search raster");
        assert!(src.starts_with("data:image/png;base64,"));
        assert!(src.len() > 80);
        assert_eq!(names_in(&home), before);
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_none(),
            "glyph raster created a file under {dir:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        match previous {
            Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
            None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
        }
    }

    fn names_in(path: &std::path::Path) -> Vec<String> {
        let mut names = Vec::new();
        let Ok(entries) = std::fs::read_dir(path) else {
            return names;
        };
        for entry in entries.flatten() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        names
    }
}
