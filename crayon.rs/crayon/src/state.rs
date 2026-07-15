use crate::{editor_state::EditorState, renderer::camera::Camera2D, resource::Resource};

/// Entire app's state.
pub struct State {
    pub camera: Camera2D,
    pub editor: EditorState,
}

impl State {
    pub fn new(window_width: u32, window_height: u32) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let camera = Camera2D::with_viewport(window_width as f32, window_height as f32);

        Self {
            camera,
            editor: EditorState::new(),
        }
    }
}

impl Resource for State {}
