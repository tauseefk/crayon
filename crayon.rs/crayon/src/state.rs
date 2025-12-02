use winit::dpi::PhysicalSize;

use crate::prelude::*;

pub struct State {
    camera: Camera2D,
    editor: EditorState,
    renderer: RendererState,
}

impl State {
    pub async fn new(window: Arc<Window>, event_sender: EventSender) -> anyhow::Result<Self> {
        let camera = Camera2D::new();
        let editor_state = EditorState::new();

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_projection(&camera);

        let initial_color = editor_state.get_brush_color_array();
        let mut renderer_state =
            RendererState::new(window.clone(), camera_uniform, initial_color, event_sender).await?;

        renderer_state.clear_render_texture();

        Ok(Self {
            camera,
            editor: editor_state,
            renderer: renderer_state,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.renderer.configure(width, height);
            #[allow(clippy::cast_precision_loss)]
            self.camera.update_aspect_ratio(width as f32, height as f32);
            self.update_camera(None);
        }
    }

    pub fn get_window_size(&self) -> PhysicalSize<u32> {
        self.renderer.window.inner_size()
    }

    pub fn clear_canvas(&mut self) {
        self.renderer.clear_render_texture();
    }

    pub fn update_camera(&mut self, transform: Option<CameraTransform>) {
        if let Some(transform) = transform {
            self.camera.update(&transform);
        }

        self.renderer.update_camera_buffer(&self.camera);
    }

    pub fn update_paint(&mut self, dot: &Dot2D) {
        self.renderer.update_paint_buffer(dot, &self.camera);
    }

    pub fn paint_to_texture(&mut self) {
        self.renderer.paint_to_texture();
    }

    pub fn update_brush_color(&mut self, color: BrushColor) {
        self.editor.update_brush_color(color);
        let color_array = self.editor.get_brush_color_array();
        self.renderer.update_brush_color(color_array);
    }

    pub fn handle_ui_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.renderer.handle_ui_event(event)
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer.render()
    }
}
