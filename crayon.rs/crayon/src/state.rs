use winit::dpi::PhysicalSize;

use crate::prelude::*;

pub struct State {
    renderer_state: RendererState,
    camera: Camera2D,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let camera = Camera2D::new();

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_projection(&camera);

        let mut renderer_state = RendererState::new(window.clone(), camera_uniform).await?;

        renderer_state.clear_render_texture();

        Ok(Self {
            renderer_state,
            camera,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.renderer_state.configure(width, height);
            self.camera.update_aspect_ratio(width as f32, height as f32);
            self.update_camera(None);
        }
    }

    pub fn get_window_size(&self) -> PhysicalSize<u32> {
        self.renderer_state.window.inner_size()
    }

    pub fn clear_canvas(&mut self) {
        self.renderer_state.clear_render_texture();
    }

    pub fn update_camera(&mut self, transform: Option<CameraTransform>) {
        if let Some(transform) = transform {
            self.camera.update(transform);
        }

        self.renderer_state.update_camera_buffer(&self.camera);
    }

    pub fn update_paint(&mut self, position: Point2<f32>) {
        self.renderer_state
            .update_paint_buffer(position, &self.camera);
    }

    pub fn paint_to_texture(&mut self) {
        self.renderer_state.paint_to_texture();
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.renderer_state.render()
    }
}
