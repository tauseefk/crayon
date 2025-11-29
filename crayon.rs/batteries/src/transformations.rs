use crate::prelude::*;

/// Converts a world position to normalized device coordinates (NDC).
#[must_use]
pub fn world_to_ndc(position: Point2<f32>, window_size: (f32, f32)) -> Point2<f32> {
    let ndc_x = (position.x / window_size.0) * 2.0 - 1.0;
    let ndc_y = 1.0 - (position.y / window_size.1) * 2.0;

    Point2::new(ndc_x, ndc_y)
}

/// Converts screen pixel position to world space coordinates.
#[must_use]
pub fn screen_to_world_position(position: Point2<f32>, window_size: (f32, f32)) -> Point2<f32> {
    let normalized_x = (position.x / window_size.0) * 2.0;
    let normalized_y = -(position.y / window_size.1) * 2.0;

    Point2::new(normalized_x, normalized_y)
}
