use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Generate a base64 thumbhash from STRAIGHT-alpha RGBA8 pixels.
///
/// thumbhash requires input <= 100x100, so the image is downscaled first,
/// preserving aspect ratio. `image::imageops::thumbnail` is a box filter —
/// adequate for a placeholder hash.
/// Only called from the ignored `generate_default_assets` test until saving
/// documents lands — hashes regenerate at save time (§6).
#[allow(dead_code)]
pub fn generate_thumbhash(rgba: &[u8], width: u32, height: u32) -> anyhow::Result<String> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .context("rgba buffer does not match width * height * 4")?;
    let (tw, th) = fit_within(width, height, 100, 100);
    let small = image::imageops::thumbnail(&img, tw, th);
    let hash = thumbhash::rgba_to_thumb_hash(tw as usize, th as usize, small.as_raw());
    Ok(BASE64.encode(hash))
}

/// Decode a base64 thumbhash to `(width, height, straight-alpha RGBA8)`.
/// Output is <= 32 px.
pub fn thumbhash_preview(hash_b64: &str) -> anyhow::Result<(usize, usize, Vec<u8>)> {
    let bytes = BASE64
        .decode(hash_b64)
        .context("thumbhash is not valid base64")?;
    thumbhash::thumb_hash_to_rgba(&bytes)
        .map_err(|()| anyhow::anyhow!("thumbhash bytes failed to decode"))
}

/// Largest size that fits within `(max_width, max_height)` while preserving
/// aspect ratio; never upscales.
#[allow(dead_code, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn fit_within(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    if width <= max_width && height <= max_height {
        return (width, height);
    }
    let scale = f64::min(
        f64::from(max_width) / f64::from(width),
        f64::from(max_height) / f64::from(height),
    );
    (
        ((f64::from(width) * scale).round() as u32).max(1),
        ((f64::from(height) * scale).round() as u32).max(1),
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
        // A simple opaque gradient — enough structure for a stable hash.
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
