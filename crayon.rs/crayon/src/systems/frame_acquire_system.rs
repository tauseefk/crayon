use crate::{
    app::App,
    prelude::*,
    renderer::render_context::RenderContext,
    system::System,
};

/// Acquires the surface texture at the start of each frame.
/// Runs in PreUpdate schedule.
pub struct FrameAcquireSystem;

impl System for FrameAcquireSystem {
    fn run(&self, app: &App) {
        let Some(mut render_ctx) = app.write::<RenderContext>() else {
            return;
        };

        // Get surface texture
        let Ok(texture) = render_ctx.surface.get_current_texture() else {
            return;
        };

        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create encoder for the frame
        let encoder = render_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        // Store for all render systems to use
        render_ctx.surface_texture = Some(texture);
        render_ctx.surface_view = Some(view);
        render_ctx.encoder = Some(encoder);
    }
}
