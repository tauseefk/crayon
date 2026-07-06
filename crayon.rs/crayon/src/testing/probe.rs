//! Coarse spatial assertions over readback pixels.
//! Should be immune to the sub-pixel and driver-level nondeterminism that makes golden-image diffs flaky.

/// RGBA8 at `(x, y)` of a tightly-packed buffer (as `readback_rgba` returns).
pub fn sample(pixels: &[u8], (width, height): (u32, u32), x: u32, y: u32) -> [u8; 4] {
    assert!(
        x < width && y < height,
        "sample({x}, {y}) outside {width}x{height}"
    );
    let index = (y * width + x) as usize * 4;
    pixels[index..index + 4].try_into().unwrap()
}

/// Panics unless every channel at `(x, y)` is within `tol` of `expect`.
pub fn assert_pixel(pixels: &[u8], size: (u32, u32), x: u32, y: u32, expect: [u8; 4], tol: u8) {
    let actual = sample(pixels, size, x, y);
    let within = actual
        .iter()
        .zip(&expect)
        .all(|(a, e)| a.abs_diff(*e) <= tol);
    assert!(
        within,
        "pixel ({x}, {y}): expected {expect:?} ±{tol}, got {actual:?}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2x2 buffer: red, green / blue, white.
    fn checker() -> Vec<u8> {
        vec![
            255, 0, 0, 255, /* */ 0, 255, 0, 255, //
            0, 0, 255, 255, /* */ 255, 255, 255, 255,
        ]
    }

    #[test]
    fn sample_indexes_by_row() {
        let pixels = checker();
        assert_eq!(sample(&pixels, (2, 2), 0, 0), [255, 0, 0, 255]);
        assert_eq!(sample(&pixels, (2, 2), 1, 0), [0, 255, 0, 255]);
        assert_eq!(sample(&pixels, (2, 2), 0, 1), [0, 0, 255, 255]);
        assert_eq!(sample(&pixels, (2, 2), 1, 1), [255, 255, 255, 255]);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn sample_rejects_out_of_bounds() {
        sample(&checker(), (2, 2), 2, 0);
    }

    #[test]
    fn assert_pixel_tolerates_within_tol() {
        let pixels = checker();
        assert_pixel(&pixels, (2, 2), 0, 0, [253, 2, 0, 255], 2);
    }

    #[test]
    #[should_panic(expected = "expected")]
    fn assert_pixel_panics_past_tol() {
        let pixels = checker();
        assert_pixel(&pixels, (2, 2), 0, 0, [252, 0, 0, 255], 2);
    }
}
