use std::fs;
use std::path::Path;

struct Slot {
    name: &'static str,
    px: u32,
    small: bool,
    preview: Option<&'static str>,
}

const SLOTS: &[Slot] = &[
    Slot {
        name: "icon_16x16.png",
        px: 16,
        small: true,
        preview: Some("Slowpage-icon-16.png"),
    },
    Slot {
        name: "icon_16x16@2x.png",
        px: 32,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_32x32.png",
        px: 32,
        small: true,
        preview: Some("Slowpage-icon-32.png"),
    },
    Slot {
        name: "icon_32x32@2x.png",
        px: 64,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_128x128.png",
        px: 128,
        small: false,
        preview: Some("Slowpage-icon-128.png"),
    },
    Slot {
        name: "icon_128x128@2x.png",
        px: 256,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_256x256.png",
        px: 256,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_256x256@2x.png",
        px: 512,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_512x512.png",
        px: 512,
        small: false,
        preview: None,
    },
    Slot {
        name: "icon_512x512@2x.png",
        px: 1024,
        small: false,
        preview: Some("Slowpage-icon-1024.png"),
    },
];

pub fn write_set_png(png: &[u8], iconset: &Path, preview: &Path) -> Result<(), String> {
    fs::create_dir_all(iconset).map_err(|err| err.to_string())?;
    fs::create_dir_all(preview).map_err(|err| err.to_string())?;
    let source = decode_non_empty_png(png)?;
    for slot in SLOTS {
        let resized = resize_pixmap(&source, slot.px)?;
        let encoded = resized.encode_png().map_err(|err| err.to_string())?;
        fs::write(iconset.join(slot.name), &encoded).map_err(|err| err.to_string())?;
        if let Some(name) = slot.preview {
            fs::write(preview.join(name), &encoded).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

pub fn resize_png(png: &[u8], px: u32) -> Result<Vec<u8>, String> {
    let source = decode_non_empty_png(png)?;
    let resized = resize_pixmap(&source, px)?;
    resized.encode_png().map_err(|err| err.to_string())
}

fn decode_non_empty_png(png: &[u8]) -> Result<resvg::tiny_skia::Pixmap, String> {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).map_err(|err| err.to_string())?;
    if pixmap.width() == 0 || pixmap.height() == 0 {
        return Err("png has no size".into());
    }
    if !pixmap.pixels().iter().any(|px| px.alpha() > 0) {
        return Err("icon png is empty".into());
    }
    Ok(pixmap)
}

pub fn resize_pixmap(
    src: &resvg::tiny_skia::Pixmap,
    px: u32,
) -> Result<resvg::tiny_skia::Pixmap, String> {
    if px == 0 {
        return Err("target icon size must be positive".into());
    }
    if src.width() == 0 || src.height() == 0 {
        return Err("source icon pixmap has no size".into());
    }

    let mut cur: Option<resvg::tiny_skia::Pixmap> = None;
    loop {
        let active = cur.as_ref().unwrap_or(src);
        if active.width() >= px * 2 && active.height() >= px * 2 {
            cur = Some(halve_pixmap(active)?);
        } else {
            break;
        }
    }

    let active = cur.as_ref().unwrap_or(src);
    let result = if active.width() == px && active.height() == px {
        match cur {
            Some(owned) => owned,
            None => active.clone(),
        }
    } else {
        let mut dst = resvg::tiny_skia::Pixmap::new(px, px).ok_or("pixmap")?;
        let paint = resvg::tiny_skia::PixmapPaint {
            quality: resvg::tiny_skia::FilterQuality::Bicubic,
            ..resvg::tiny_skia::PixmapPaint::default()
        };
        let transform = resvg::tiny_skia::Transform::from_scale(
            px as f32 / active.width() as f32,
            px as f32 / active.height() as f32,
        );
        dst.draw_pixmap(0, 0, active.as_ref(), &paint, transform, None);
        dst
    };

    if !result.pixels().iter().any(|p| p.alpha() > 0) {
        return Err("icon raster is empty".into());
    }
    Ok(result)
}

fn halve_pixmap(src: &resvg::tiny_skia::Pixmap) -> Result<resvg::tiny_skia::Pixmap, String> {
    let dst_w = src.width() / 2;
    let dst_h = src.height() / 2;
    if dst_w == 0 || dst_h == 0 {
        return Err("cannot halve pixmap below 1px".into());
    }
    let src_w = src.width() as usize;
    let src_pixels = src.pixels();
    let mut dst = resvg::tiny_skia::Pixmap::new(dst_w, dst_h).ok_or("pixmap")?;
    let dst_pixels = dst.pixels_mut();

    for y in 0..dst_h as usize {
        let sy = y * 2;
        for x in 0..dst_w as usize {
            let sx = x * 2;
            let p00 = src_pixels[sy * src_w + sx];
            let p10 = src_pixels[sy * src_w + sx + 1];
            let p01 = src_pixels[(sy + 1) * src_w + sx];
            let p11 = src_pixels[(sy + 1) * src_w + sx + 1];

            let r = ((u16::from(p00.red())
                + u16::from(p10.red())
                + u16::from(p01.red())
                + u16::from(p11.red())
                + 2)
                / 4) as u8;
            let g = ((u16::from(p00.green())
                + u16::from(p10.green())
                + u16::from(p01.green())
                + u16::from(p11.green())
                + 2)
                / 4) as u8;
            let b = ((u16::from(p00.blue())
                + u16::from(p10.blue())
                + u16::from(p01.blue())
                + u16::from(p11.blue())
                + 2)
                / 4) as u8;
            let a = ((u16::from(p00.alpha())
                + u16::from(p10.alpha())
                + u16::from(p01.alpha())
                + u16::from(p11.alpha())
                + 2)
                / 4) as u8;

            dst_pixels[y * dst_w as usize + x] =
                resvg::tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a)
                    .ok_or("invalid premultiplied pixel")?;
        }
    }

    Ok(dst)
}

pub fn write_set(svg: &str, iconset: &Path, preview: &Path) -> Result<(), String> {
    fs::create_dir_all(iconset).map_err(|err| err.to_string())?;
    fs::create_dir_all(preview).map_err(|err| err.to_string())?;
    let small = small_variant(svg)?;
    for slot in SLOTS {
        let source = if slot.small { small.as_str() } else { svg };
        let png = raster_png(source, slot.px)?;
        fs::write(iconset.join(slot.name), &png).map_err(|err| err.to_string())?;
        if let Some(name) = slot.preview {
            fs::write(preview.join(name), &png).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

pub fn raster_png(svg: &str, px: u32) -> Result<Vec<u8>, String> {
    let pixmap = raster(svg, px)?;
    pixmap.encode_png().map_err(|err| err.to_string())
}

pub fn raster(svg: &str, px: u32) -> Result<resvg::tiny_skia::Pixmap, String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|err| err.to_string())?;
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
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let ink = pixmap.pixels().iter().filter(|px| px.alpha() > 0).count();
    if ink == 0 {
        return Err("icon raster is empty".into());
    }
    Ok(pixmap)
}

/// 16 and 32 point @1x. Strips fine cadence lines and ribbon details that blur
/// below half a pixel, keeping the warm squircle and open turning pages crisp.
pub fn small_variant(svg: &str) -> Result<String, String> {
    let start_mark = "<g id=\"detail-fine\">";
    let end_mark = "</g>";
    let start = svg
        .find(start_mark)
        .ok_or_else(|| "icon svg is missing #detail-fine".to_string())?;
    let after = start + start_mark.len();
    let rel_end = svg[after..]
        .find(end_mark)
        .ok_or_else(|| "truncated #detail-fine group".to_string())?;
    let end = after + rel_end + end_mark.len();
    let mut out = String::with_capacity(svg.len());
    out.push_str(&svg[..start]);
    out.push_str(&svg[end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const PNG: &[u8] = include_bytes!("../../../assets/icon/Slowpage.png");

    #[test]
    fn source_is_1024_transparent_png() {
        let pixmap = decode_non_empty_png(PNG).unwrap();
        assert_eq!(pixmap.width(), 1024);
        assert_eq!(pixmap.height(), 1024);
        assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);
        assert_eq!(pixmap.pixel(1023, 0).unwrap().alpha(), 0);
        assert_eq!(pixmap.pixel(0, 1023).unwrap().alpha(), 0);
        assert_eq!(pixmap.pixel(1023, 1023).unwrap().alpha(), 0);
    }

    fn tally(
        pixmap: &resvg::tiny_skia::Pixmap,
        pred: fn(&resvg::tiny_skia::PremultipliedColorU8) -> bool,
    ) -> usize {
        pixmap.pixels().iter().filter(|pixel| pred(pixel)).count()
    }

    fn amber_glass(pixel: &resvg::tiny_skia::PremultipliedColorU8) -> bool {
        pixel.alpha() > 200
            && pixel.red() > 170
            && pixel.green() > 55
            && pixel.red() > pixel.blue().saturating_add(30)
    }

    fn white_paper(pixel: &resvg::tiny_skia::PremultipliedColorU8) -> bool {
        pixel.alpha() > 220 && pixel.red() > 220 && pixel.green() > 220 && pixel.blue() > 220
    }

    #[test]
    fn resized_slots_preserve_amber_glass_and_white_paper() {
        let source = decode_non_empty_png(PNG).unwrap();
        let p128 = resize_pixmap(&source, 128).unwrap();
        assert_eq!(p128.pixel(0, 0).unwrap().alpha(), 0);
        assert!(tally(&p128, amber_glass) > 600);
        assert!(tally(&p128, white_paper) > 3000);

        let p32 = resize_pixmap(&source, 32).unwrap();
        assert_eq!(p32.pixel(0, 0).unwrap().alpha(), 0);
        assert!(tally(&p32, amber_glass) > 40);
        assert!(tally(&p32, white_paper) > 180);
    }

    #[test]
    fn write_set_png_populates_iconset_and_previews() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("spike-icon-test-{stamp}"));
        let iconset = tmp.join("AppIcon.iconset");
        let preview = tmp.join("preview");

        write_set_png(PNG, &iconset, &preview).unwrap();

        for slot in SLOTS {
            let bytes = fs::read(iconset.join(slot.name)).unwrap();
            let decoded = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
            assert_eq!(decoded.width(), slot.px);
            assert_eq!(decoded.height(), slot.px);
            if let Some(prev_name) = slot.preview {
                let prev_bytes = fs::read(preview.join(prev_name)).unwrap();
                let prev_decoded = resvg::tiny_skia::Pixmap::decode_png(&prev_bytes).unwrap();
                assert_eq!(prev_decoded.width(), slot.px);
                assert_eq!(prev_decoded.height(), slot.px);
            }
        }

        let _ = fs::remove_dir_all(&tmp);
    }
}
