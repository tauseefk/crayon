use crate::{
    app::App,
    renderer::{frame_context::FrameContext, render_context::RenderContext},
    resource::ResourceContext,
    resources::{document_state::DocumentState, scene_renderer::SceneRenderer},
    state::State,
    system::System,
};

/// Composites the document — artboard backgrounds and layers as world-space
/// quads — to the surface through the camera (multi-artboard.md §2.7).
pub struct CanvasRenderSystem;

impl System for CanvasRenderSystem {
    fn run(&self, app: &App) {
        let (Some(mut render_ctx), Some(frame_ctx), Some(mut scene), Some(doc), Some(state)) = (
            app.write::<RenderContext>(),
            app.read::<FrameContext>(),
            app.write::<SceneRenderer>(),
            app.read::<DocumentState>(),
            app.read::<State>(),
        ) else {
            return;
        };

        let render_ctx = &mut *render_ctx;
        let Some(view) = frame_ctx.surface_view.as_ref() else {
            return;
        };
        let Some(encoder) = render_ctx.encoder.as_mut() else {
            return;
        };

        scene.render(
            &render_ctx.device,
            &render_ctx.queue,
            encoder,
            view,
            (render_ctx.config.width, render_ctx.config.height),
            &doc.document,
            &state.camera,
        );
    }
}
