use crate::{
    app::App,
    renderer::render_context::RenderContext,
    resource::ResourceContext,
    resources::{
        brush_point_queue::BrushPointQueue,
        document_state::{DocumentState, GpuOp},
        scene_renderer::SceneRenderer,
        stroke_state::StrokeState,
    },
    system::System,
};

/// Applies queued structural `GpuOp`s to the `SceneRenderer` before any pass
/// is recorded (multi-artboard.md §2.3).
///
/// Stroke accumulation and merge return in S3
/// (multi-artboard-implementation.md); until then queued brush points and
/// stroke flags are drained and dropped so painting state can't leak across
/// the stage boundary.
pub struct PaintSystem;

impl System for PaintSystem {
    fn run(&self, app: &App) {
        let (
            Some(render_ctx),
            Some(mut scene),
            Some(mut doc),
            Some(mut brush_point_queue),
            Some(mut stroke_state),
        ) = (
            app.read::<RenderContext>(),
            app.write::<SceneRenderer>(),
            app.write::<DocumentState>(),
            app.write::<BrushPointQueue>(),
            app.write::<StrokeState>(),
        )
        else {
            return;
        };

        for op in doc.gpu_dirty.drain(..) {
            match op {
                GpuOp::ClearLayer { layer } => {
                    scene.clear_layer(&render_ctx.device, &render_ctx.queue, layer);
                }
            }
        }

        while brush_point_queue.read().is_some() {}
        let _ = stroke_state.take_needs_clear();
        let _ = stroke_state.take_needs_merge();
    }
}
