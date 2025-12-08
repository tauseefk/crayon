use crate::{
    app::App,
    prelude::*,
    renderer::{frame_context::FrameContext, render_context::RenderContext},
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
        let Some(mut frame_ctx) = app.write::<FrameContext>() else {
            return;
        };

        // Get surface texture
        let Ok(texture) = render_ctx.surface.get_current_texture() else {
            return;
        };

        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create encoder
        let encoder = render_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        // Store in separate resources
        frame_ctx.surface_texture = Some(texture);
        frame_ctx.surface_view = Some(view);
        render_ctx.encoder = Some(encoder);
    }
}
