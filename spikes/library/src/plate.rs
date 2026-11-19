//! Fixture plates carry `slowpage-plate` plus six hex digits. A photo cover
//! does not, so the shelf can tell a generated plate from a real image.

use std::io::Cursor;

const MARKER: &[u8] = b"slowpage-plate\0";

/// `RRGGBB` stored in a plate PNG, or none when the bytes are a photo.
pub fn plate_color(bytes: &[u8]) -> Option<u32> {
    let index = bytes
        .windows(MARKER.len())
        .position(|window| window == MARKER)?;
    let start = index + MARKER.len();
    let hex = bytes.get(start..start + 6)?;
    let text = std::str::from_utf8(hex).ok()?;
    u32::from_str_radix(text, 16).ok()
}

/// Center pixel of a PNG, used when a PDF raster has no plate marker.
pub fn png_center_rgb(bytes: &[u8]) -> Option<u32> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let width = info.width as usize;
    let height = info.height as usize;
    if width == 0 || height == 0 {
        return None;
    }
    let channels = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    let x = width / 2;
    let y = height / 2;
    let index = (y * width + x) * channels;
    let pixel = buf.get(index..index + 3)?;
    Some((u32::from(pixel[0]) << 16) | (u32::from(pixel[1]) << 8) | u32::from(pixel[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_is_the_plate_color_and_a_photo_is_not() {
        let mut bytes = b"....slowpage-plate".to_vec();
        bytes.push(0);
        bytes.extend(b"7D8C6A....");
        assert_eq!(plate_color(&bytes), Some(0x7D8C6A));
        assert_eq!(plate_color(b"plain essay cover"), None);
    }
}
