use crate::{
    app::App,
    prelude::*,
    renderer::render_context::RenderContext,
    resources::{brush_point_queue::BrushPointQueue, canvas_state::CanvasContext},
    system::System,
};

/// Paints queued brush points to render texture.
pub struct PaintSystem;

impl System for PaintSystem {
    fn run(&self, app: &App) {
        let Some(render_ctx) = app.read::<RenderContext>() else {
            return;
        };
        let Some(mut canvas_ctx) = app.write::<CanvasContext>() else {
            return;
        };
        let Some(mut brush_point_queue) = app.write::<BrushPointQueue>() else {
            return;
        };

        // Renders each point to a texture, and then swaps the textures.
        // Submits once per point.
        while let Some(point_data) = brush_point_queue.read() {
            canvas_ctx.update_paint_buffer(&render_ctx, &point_data.dot, &point_data.camera);

            let (read_bind_group, write_texture_view) = if canvas_ctx.is_rendering_to_a {
                (
                    &canvas_ctx.paint_fragment_bind_group_b,
                    &canvas_ctx.render_texture_a.view,
                )
            } else {
                (
                    &canvas_ctx.paint_fragment_bind_group_a,
                    &canvas_ctx.render_texture_b.view,
                )
            };

            let mut encoder =
                render_ctx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Paint Encoder"),
                    });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Paint Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: write_texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&canvas_ctx.paint_pipeline);
                render_pass.set_bind_group(0, &canvas_ctx.paint_uniform_bind_group, &[]);
                render_pass.set_bind_group(1, read_bind_group, &[]);
                render_pass.set_vertex_buffer(0, canvas_ctx.camera_vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    canvas_ctx.camera_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                render_pass.draw_indexed(0..canvas_ctx.index_count, 0, 0..1);
            }

            render_ctx.queue.submit(std::iter::once(encoder.finish()));
            canvas_ctx.is_rendering_to_a = !canvas_ctx.is_rendering_to_a;
        }
    }
}
