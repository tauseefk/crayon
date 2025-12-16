use crate::constants::CAMERA_ZOOM_DELTA;

/// Get zoom delta based on scroll y value
pub fn get_zoom_delta(scroll_y: f32) -> f32 {
    if scroll_y > 0.0 {
        CAMERA_ZOOM_DELTA
    } else if scroll_y < 0.0 {
        -CAMERA_ZOOM_DELTA
    } else {
        0.0
    }
}
