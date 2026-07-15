use crate::{
    app::App,
    renderer::render_context::RenderContext,
    resource::ResourceContext,
    resources::{
        brush_point_queue::BrushPointQueue,
        brush_preview_state::BrushPreviewState,
        document_state::{DocumentState, GpuOp},
        scene_renderer::{DabInstance, SceneRenderer},
        stroke_state::StrokeState,
    },
    system::System,
};

/// Accumulates queued brush points into the stroke scratch, targeting the
/// stroke's layer (multi-artboard.md §2.5).
///
/// Structural `GpuOp`s are drained first — before any pass is recorded — so
/// scratch reallocation never happens mid-stroke. The whole frame's dabs are
/// stamped in one instanced pass; the stroke merges into its layer once, on
/// stroke end.
pub struct PaintSystem;

impl System for PaintSystem {
    fn run(&self, app: &App) {
        let (
            Some(mut render_ctx),
            Some(mut scene),
            Some(mut doc),
            Some(mut brush_point_queue),
            Some(mut preview_state),
            Some(mut stroke_state),
        ) = (
            app.write::<RenderContext>(),
            app.write::<SceneRenderer>(),
            app.write::<DocumentState>(),
            app.write::<BrushPointQueue>(),
            app.write::<BrushPreviewState>(),
            app.write::<StrokeState>(),
        )
        else {
            return;
        };
        let render_ctx = &mut *render_ctx;

        // 0. structural ops first — never mid-stroke reallocation (§2.3)
        for op in doc.gpu_dirty.drain(..) {
            match op {
                GpuOp::ClearLayer { layer } => {
                    scene.clear_layer(&render_ctx.device, &render_ctx.queue, layer);
                }
            }
        }

        // 1. drain the queue → dabs in the target layer's clip space:
        //    screen px → world px (per-point camera snapshot) → layer-local px → layer clip
        let mut last_position = None;
        {
            let dabs = scene.begin_dabs();
            while let Some(point) = brush_point_queue.read() {
                last_position = Some(point.dot.position);

                let Some((artboard_id, layer_id)) = point.target else {
                    continue;
                };
                // Skip points whose artboard/layer was deleted mid-flight.
                let Some(artboard) = doc.document.artboard(artboard_id) else {
                    continue;
                };
                let Some(layer) = artboard.layers.iter().find(|layer| layer.id == layer_id)
                else {
                    continue;
                };

                let world = point.camera.screen_to_world(point.dot.position);
                let local_x = world.x - artboard.position[0] - layer.offset[0];
                let local_y = world.y - artboard.position[1] - layer.offset[1];
                #[allow(clippy::cast_precision_loss)]
                let (width, height) = {
                    let (w, h) = artboard.pixel_size();
                    (w as f32, h as f32)
                };

                dabs.push(DabInstance {
                    center: [
                        local_x / (width * 0.5) - 1.0,
                        1.0 - local_y / (height * 0.5),
                    ],
                    radius_px: point.dot.radius,
                });
            }
        }

        // The brush preview follows the cursor in screen px.
        if let Some(position) = last_position {
            preview_state.show_at_position(position);
        }

        let clear = stroke_state.take_needs_clear();
        let merge = stroke_state.take_needs_merge();

        let instance_count = scene.upload_dabs(&render_ctx.queue);

        if instance_count == 0 && !clear && !merge {
            return;
        }

        // GPU work only while the target layer still exists — a delete
        // mid-stroke aborts accumulation and skips the merge.
        let target_layer = stroke_state
            .target
            .and_then(|(_, layer_id)| scene.layers.get(&layer_id).map(|layer| (layer_id, layer.size)));
        let Some((layer_id, layer_size)) = target_layer else {
            return;
        };

        let Some(encoder) = render_ctx.encoder.as_mut() else {
            return;
        };

        if clear || instance_count > 0 {
            scene.accumulate_stroke(&render_ctx.queue, encoder, clear, instance_count, layer_size);
        }

        if merge {
            scene.merge_stroke_into_layer(&render_ctx.queue, encoder, layer_id);
        }
    }
}
