use crate::{
    app::App,
    renderer::render_context::RenderContext,
    resource::ResourceContext,
    resources::{
        brush_point_queue::BrushPointQueue,
        brush_preview_state::BrushPreviewState,
        document_state::{DocumentState, GpuOp},
        scene_renderer::{PointInstance, SceneRenderer},
        stroke_state::StrokeState,
    },
    system::System,
};

/// Applies queued structural `GpuOp`s to the `SceneRenderer` before any pass is recorded.
///
// TODO: Stroke accumulation and merge.
// Until then queued brush points and stroke flags are drained and dropped so painting state can't leak across the stage boundary.
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

        // this avoids mid-stroke allocations
        for op in doc.gpu_dirty.drain(..) {
            match op {
                GpuOp::ClearLayer { layer_id: layer } => {
                    scene.clear_layer(&render_ctx.device, &render_ctx.queue, layer);
                }
            }
        }

        let mut last_position = None;
        {
            let points = scene.begin_points();
            while let Some(point) = brush_point_queue.read() {
                last_position = Some(point.dot.position);

                let Some((artboard_id, layer_id)) = point.target else {
                    continue;
                };

                let Some(artboard) = doc.document.artboard(artboard_id) else {
                    continue;
                };

                let Some(layer) = artboard.layer(layer_id) else {
                    continue;
                };

                let world = point.camera.screen_to_world(point.dot.position);
                let (local_x, local_y) = (
                    world.x - artboard.position[0] - layer.offset[0],
                    world.y - artboard.position[1] - layer.offset[1],
                );

                let (width, height) = {
                    let (w, h) = artboard.pixel_size();
                    (w as f32, h as f32)
                };

                points.push(PointInstance {
                    center: [
                        local_x / (width * 0.5) - 1.0,
                        1.0 - local_y / (height * 0.5),
                    ],
                    radius_px: point.dot.radius,
                })
            }
        }

        if let Some(position) = last_position {
            preview_state.show_at_position(position);
        }

        let needs_clear = stroke_state.take_needs_clear();
        let needs_merge = stroke_state.take_needs_merge();

        let instance_count = scene.upload_points(&render_ctx.queue);
        if instance_count == 0 && !needs_clear && !needs_merge {
            return;
        }

        let target_layer = stroke_state.target.and_then(|(_, layer_id)| {
            scene
                .layers
                .get(&layer_id)
                .map(|layer| (layer_id, layer.size))
        });
        let Some((layer_id, layer_size)) = target_layer else {
            return;
        };

        let Some(encoder) = render_ctx.encoder.as_mut() else {
            return;
        };

        if needs_clear || instance_count > 0 {
            scene.accumulate_stroke(
                &render_ctx.queue,
                encoder,
                needs_clear,
                instance_count,
                layer_size,
            );
        }

        if needs_merge {
            scene.merge_stroke_into_layer(&render_ctx.queue, encoder, layer_id);
        }
    }
}
