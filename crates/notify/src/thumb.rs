//! Thumbnails for cards (card #149): a PNG on this device, decoded and
//! shrunk here so the card thread only ever scales a small picture.
//!
//! Only local files: the resolver ([`crate::FileFn`]) answers from this
//! device's attachment store, and a file that isn't here gives no
//! thumbnail (the card looks as it did without one). Nothing here fetches
//! over the mesh, so it is safe on the Watch thread.

use std::io::Cursor;
use std::path::Path;

use toast::Image;

/// The largest thumbnail kept, in px: twice the card's slot (2 px per
/// dip), so it is sharp on a Retina or 200% screen and never bigger.
pub const THUMB_MAX_W: u32 = (toast::layout::TEXT_WIDTH_DIP * 2.0) as u32;
pub const THUMB_MAX_H: u32 = (toast::layout::THUMB_MAX_H_DIP * 2.0) as u32;

/// Files past this are skipped, not read (a screenshot is far smaller).
pub const MAX_FILE_BYTES: u64 = 32 << 20;
/// What the decoder may allocate: a 4K RGBA screenshot is about 33 MiB.
const DECODE_LIMIT: usize = 128 << 20;

/// The PNG at path as a thumbnail, or `None` when it is missing, too
/// big, or not a PNG.
pub fn thumbnail(path: &Path) -> Option<Image> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    thumbnail_from_png(&bytes)
}

/// A thumbnail from PNG bytes: any PNG colour type and depth becomes RGBA8,
/// shrunk to fit [`THUMB_MAX_W`] x [`THUMB_MAX_H`] with its aspect kept.
pub fn thumbnail_from_png(bytes: &[u8]) -> Option<Image> {
    let (w, h, rgba) = decode_png(bytes)?;
    let (tw, th) = fit(w, h, THUMB_MAX_W, THUMB_MAX_H);
    let rgba = if (tw, th) == (w, h) {
        rgba
    } else {
        shrink(&rgba, w, h, tw, th)
    };
    Image::new(tw, th, rgba)
}

/// A thumbnail for the attachment with this sha256, when this device has
/// it (`file` resolves it to a local path).
pub fn attachment_thumbnail(file: Option<&crate::FileFn>, sha256: &str) -> Option<Image> {
    let path = file?(sha256)?;
    thumbnail(&path)
}

fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut d = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: DECODE_LIMIT,
        },
    );
    d.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = w as usize * h as usize;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None, // EXPAND turns these into Rgb(a)
    };
    (rgba.len() == px * 4 && px > 0).then_some((w, h, rgba))
}

/// w x h scaled to fit max_w x max_h, aspect kept, never scaled up, at
/// least 1 x 1.
pub fn fit(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w <= max_w && h <= max_h {
        return (w, h);
    }
    let s = (max_w as f64 / w as f64).min(max_h as f64 / h as f64);
    (
        ((w as f64 * s).round() as u32).clamp(1, max_w),
        ((h as f64 * s).round() as u32).clamp(1, max_h),
    )
}

/// An area-average (box filter) shrink of straight RGBA: each output pixel
/// averages the source pixels it covers, colour weighted by alpha so
/// transparent pixels don't darken the edges.
fn shrink(src: &[u8], w: u32, h: u32, tw: u32, th: u32) -> Vec<u8> {
    let (w, h, tw, th) = (w as usize, h as usize, tw as usize, th as usize);
    let mut out = vec![0u8; tw * th * 4];
    for ty in 0..th {
        let y0 = ty * h / th;
        let y1 = ((ty + 1) * h / th).max(y0 + 1);
        for tx in 0..tw {
            let x0 = tx * w / tw;
            let x1 = ((tx + 1) * w / tw).max(x0 + 1);
            let (mut r, mut g, mut b, mut a, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for y in y0..y1 {
                let row = &src[(y * w + x0) * 4..(y * w + x1) * 4];
                for p in row.chunks_exact(4) {
                    let pa = p[3] as u64;
                    r += p[0] as u64 * pa;
                    g += p[1] as u64 * pa;
                    b += p[2] as u64 * pa;
                    a += pa;
                    n += 1;
                }
            }
            let o = &mut out[(ty * tw + tx) * 4..][..4];
            // Fully transparent (a == 0) stays black: its colour never shows.
            for (ch, c) in o.iter_mut().zip([r, g, b]) {
                *ch = (c + a / 2).checked_div(a).unwrap_or(0) as u8;
            }
            o[3] = ((a + n / 2) / n) as u8;
        }
    }
    out
}
