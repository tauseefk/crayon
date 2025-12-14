use crate::prelude::*;

/// Converts world space coordinates to screen pixel position.
///
/// x ranges  0.0 to window_width
/// y ranges  0.0 to window_height
#[must_use]
pub const fn world_to_screen_position(
    position: Point2<f32>,
    window_size: (f32, f32),
) -> Point2<f32> {
    let pixel_x = (position.x / 2.0) * window_size.0;
    let pixel_y = (-position.y / 2.0) * window_size.1;

    Point2::new(pixel_x, pixel_y)
}

/// Converts screen pixel position to world space coordinates.
///
/// x ranges from 0.0 to 2.0 (left to right)
/// y ranges from 0.0 to -2.0 (top to bottom)
#[must_use]
pub const fn screen_to_world_position(
    position: Point2<f32>,
    window_size: (f32, f32),
) -> Point2<f32> {
    let normalized_x = (position.x / window_size.0) * 2.0;
    let normalized_y = -(position.y / window_size.1) * 2.0;

    Point2::new(normalized_x, normalized_y)
}

/// Converts a screen pixel position to normalized device coordinates (NDC).
///
/// x ranges from -1.0 to 1.0 (left to right)
/// y ranges from 1.0 to -1.0 (top to bottom)
#[must_use]
pub const fn screen_to_ndc(position: Point2<f32>, window_size: (f32, f32)) -> Point2<f32> {
    let ndc_x = (position.x / window_size.0) * 2.0 - 1.0;
    let ndc_y = 1.0 - (position.y / window_size.1) * 2.0;

    Point2::new(ndc_x, ndc_y)
}
