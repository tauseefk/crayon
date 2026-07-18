use cgmath::{EuclideanSpace, Point2};

use crate::{editor_state::EditorState, renderer::camera::Camera2D, resource::Resource};

/// Entire app's state.
pub struct State {
    pub camera: Camera2D,
    pub editor: EditorState,
    /// Last accumulated pan offset received from the camera controller.
    /// Consecutive `CameraMove` payloads are turned into deltas against accumulated offset
    /// which ensures panning stays consistent at any zoom level.
    pub pan_offset: Point2<f32>,
}

impl State {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        let camera = Camera2D::with_viewport(window_width as f32, window_height as f32);

        Self {
            camera,
            editor: EditorState::new(),
            pan_offset: Point2::origin(),
        }
    }
}

impl Resource for State {}
