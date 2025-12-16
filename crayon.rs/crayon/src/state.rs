use crate::{
    editor_state::EditorState,
    renderer::camera::{Camera2D, CameraUniform},
    resource::Resource,
};

/// Entire app's state.
pub struct State {
    pub camera: Camera2D,
    pub editor: EditorState,
}

impl State {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        let camera = Camera2D::with_viewport(window_width as f32, window_height as f32);
        let editor_state = EditorState::new();

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_projection(&camera);

        Self {
            camera,
            editor: editor_state,
        }
    }
}

impl Resource for State {}
