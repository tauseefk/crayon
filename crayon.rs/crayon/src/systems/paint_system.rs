use cgmath::Point2;

use crate::{
    app::App,
    renderer::render_context::RenderContext,
    resource::ResourceContext,
    resources::{
        brush_point_queue::BrushPointQueue,
        brush_preview_state::BrushPreviewState,
        canvas_state::{CanvasContext, DabInstance},
        stroke_state::StrokeState,
    },
    system::System,
};

/// Accumulates queued brush points into the stroke layer.
///
/// The whole frame's dabs are stamped in a single instanced pass instead of one
/// full-canvas pass + submit per point. The stroke layer is merged into the canvas once,
/// on stroke end.
pub struct PaintSystem;

impl System for PaintSystem {
    fn run(&self, app: &App) {
        let (
            Some(mut render_ctx),
            Some(mut canvas_ctx),
            Some(mut brush_point_queue),
            Some(mut preview_state),
            Some(mut stroke_state),
        ) = (
            app.write::<RenderContext>(),
            app.write::<CanvasContext>(),
            app.write::<BrushPointQueue>(),
            app.write::<BrushPreviewState>(),
            app.write::<StrokeState>(),
        )
        else {
            return;
        };

        // Drain the queue into the reused per-instance dab staging buffer. Each dab's
        // center is converted from screen NDC into canvas NDC using the camera captured
        // when the point arrived.
        let mut last_position = None;
        {
            let dabs = canvas_ctx.begin_dabs();
            while let Some(point_data) = brush_point_queue.read() {
                let position = point_data.dot.position;
                last_position = Some(position);

                let inverse_view_projection = point_data.camera.build_2d_inverse_transform_matrix();
                let center = inverse_view_projection
                    * cgmath::Vector4::new(position.x, position.y, 0.0, 1.0);

                dabs.push(DabInstance {
                    center: [center.x, center.y],
                    radius: point_data.dot.radius,
                });
            }
        }

        if let Some(position) = last_position {
            // transformation: [-1, 1] -> [0, 2]
            preview_state.show_at_position(Point2 {
                x: position.x + 1.,
                y: position.y - 1.,
            });
        }

        let clear = stroke_state.take_needs_clear();
        let merge = stroke_state.take_needs_merge();

        let instance_count = canvas_ctx.upload_dabs(&render_ctx);

        if instance_count == 0 && !clear && !merge {
            return;
        }

        let Some(encoder) = render_ctx.encoder.as_mut() else {
            return;
        };

        if clear || instance_count > 0 {
            canvas_ctx.accumulate_stroke(encoder, clear, instance_count);
        }

        if merge {
            canvas_ctx.record_merge_and_clear(encoder);
        }
    }
}
