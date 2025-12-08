use crate::{
    app::App,
    prelude::*,
    renderer::{frame_context::FrameContext, render_context::RenderContext},
    system::System,
};

/// Presents the frame and cleans up.
/// Runs in PostUpdate schedule.
pub struct FramePresentSystem;

impl System for FramePresentSystem {
    fn run(&self, app: &App) {
        let Some(mut render_ctx) = app.write::<RenderContext>() else {
            return;
        };
        let Some(mut frame_ctx) = app.write::<FrameContext>() else {
            return;
        };

        // Submit the encoder
        if let Some(encoder) = render_ctx.encoder.take() {
            render_ctx.queue.submit(std::iter::once(encoder.finish()));
        }

        // Present the surface texture
        if let Some(texture) = frame_ctx.surface_texture.take() {
            texture.present();
        }

        // Clear the view
        frame_ctx.surface_view = None;
    }
}
