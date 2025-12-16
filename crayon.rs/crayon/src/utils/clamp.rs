use cgmath::Point2;

use crate::constants::{CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN};

const TRANSLATION_MIN_X: f32 = -1.0;
const TRANSLATION_MAX_X: f32 = 1.0;
const TRANSLATION_MIN_Y: f32 = -1.0;
const TRANSLATION_MAX_Y: f32 = 1.0;

/// Clamp the translation so the canvas or tool doesn't move out of the viewport
pub fn clamp_point(translation: Point2<f32>) -> Point2<f32> {
    Point2 {
        x: translation.x.clamp(TRANSLATION_MIN_X, TRANSLATION_MAX_X),
        y: translation.y.clamp(TRANSLATION_MIN_Y, TRANSLATION_MAX_Y),
    }
}

/// Clamp the zoom so the zoom value doesn't get out of hand
pub fn clamp_zoom(current_zoom: f32, delta: f32) -> f32 {
    (current_zoom + delta).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX)
}
