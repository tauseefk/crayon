use crate::constants::{CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN};

/// Clamp the zoom so the zoom value doesn't get out of hand
pub fn clamp_zoom(current_zoom: f32, delta: f32) -> f32 {
    (current_zoom + delta).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX)
}
