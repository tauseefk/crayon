use crate::{
    app::App,
    renderer::{frame_context::FrameContext, render_context::RenderContext},
    resource::ResourceContext,
    system::System,
};

/// Presents the frame and cleans up.
/// This should run at the end of each frame after systems that render to screen.
pub struct FramePresentSystem;

impl System for FramePresentSystem {
    fn run(&self, app: &App) {
        let (Some(mut render_ctx), Some(mut frame_ctx)) =
            (app.write::<RenderContext>(), app.write::<FrameContext>())
        else {
            return;
        };

        if let Some(encoder) = render_ctx.encoder.take() {
            render_ctx.queue.submit(std::iter::once(encoder.finish()));
        }

        if let Some(texture) = frame_ctx.surface_texture.take() {
            texture.present();
        }

        frame_ctx.surface_view = None;
    }
}
