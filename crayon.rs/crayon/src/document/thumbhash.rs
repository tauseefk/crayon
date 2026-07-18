use anyhow::Context;
use base64::{Engine, engine::general_purpose};

/// Generate a base64 thumbhash from straigh-alpha RGBA8 pixels.
///
/// thumbhash requires input <= 100x100, so the image is downscaled first, preserving aspect ratio.
/// `image::imageops::thumbnail` is a box filter, adequate for a placeholder hash.
pub fn generate_thumbhash(rgba: &[u8], width: u32, height: u32) -> anyhow::Result<String> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .context("rgba buffer does not match the expected size")?;
    let (tw, th) = fit_within(width, height, 100, 100);
    let small = image::imageops::thumbnail(&img, tw, th);
    let hash = thumbhash::rgba_to_thumb_hash(tw as usize, th as usize, small.as_raw());

    Ok(general_purpose::STANDARD.encode(hash))
}

/// Decode a base64 thumbhash to `(width, height, straight-alpha RGBA8)`.
/// Output is <= 32 px.
pub fn thumbhash_preview(hash_b64: &str) -> anyhow::Result<(usize, usize, Vec<u8>)> {
    let bytes = general_purpose::STANDARD
        .decode(hash_b64)
        .context("thumbhash is not valid base64")?;
    thumbhash::thumb_hash_to_rgba(&bytes)
        .map_err(|()| anyhow::anyhow!("thumbhash bytes failed to decode"))
}

/// Returns the largest size that fits within the `(max_w, max_h)` while preserving the aspect ratio.
/// Does not upscale.
fn fit_within(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w <= max_w && h <= max_h {
        return (w, h);
    }

    let scale = f64::min(
        f64::from(max_w) / f64::from(w),
        f64::from(max_h) / f64::from(h),
    );
    (
        ((f64::from(w) * scale).round() as u32).max(1),
        ((f64::from(h) * scale).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_within_preserves_aspect_and_bounds() {
        assert_eq!(fit_within(80, 60, 100, 100), (80, 60));
        assert_eq!(fit_within(800, 600, 100, 100), (100, 75));
        assert_eq!(fit_within(600, 800, 100, 100), (75, 100));
        assert_eq!(fit_within(4000, 10, 100, 100), (100, 1));
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn thumbhash_round_trip() {
        // A simple opaque gradient, enough structure for a stable hash.
        let (w, h) = (64u32, 48u32);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[(x * 4) as u8, (y * 5) as u8, 128, 255]);
            }
        }
        let hash = generate_thumbhash(&rgba, w, h).unwrap();
        let (pw, ph, pixels) = thumbhash_preview(&hash).unwrap();
        assert!(pw > 0 && ph > 0);
        assert_eq!(pixels.len(), pw * ph * 4);
        // Fully opaque input decodes to a fully opaque placeholder.
        assert!(pixels.chunks_exact(4).all(|px| px[3] == 255));
    }

    #[test]
    fn generate_rejects_mismatched_buffer() {
        assert!(generate_thumbhash(&[0u8; 12], 4, 4).is_err());
    }

    #[test]
    fn preview_rejects_garbage() {
        assert!(thumbhash_preview("not base64!!!").is_err());
        assert!(thumbhash_preview("AAAA").is_err());
    }
}
