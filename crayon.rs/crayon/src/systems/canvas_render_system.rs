use crate::{
    app::App,
    prelude::*,
    renderer::{frame_context::FrameContext, render_context::RenderContext},
    resources::canvas_state::CanvasContext,
    system::System,
};

/// Renders the canvas to the surface using the camera pipeline.
pub struct CanvasRenderSystem;

impl System for CanvasRenderSystem {
    fn run(&self, app: &App) {
        let Some(mut render_ctx) = app.write::<RenderContext>() else {
            return;
        };
        let Some(frame_ctx) = app.read::<FrameContext>() else {
            return;
        };
        let Some(canvas) = app.read::<CanvasContext>() else {
            return;
        };

        // Get view from FrameContext (immutable) and encoder from RenderContext (mutable)
        let Some(view) = frame_ctx.surface_view.as_ref() else {
            return;
        };
        let Some(encoder) = render_ctx.encoder.as_mut() else {
            return;
        };

        // Render canvas to surface
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Canvas Display Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let camera_bind_group = canvas.get_camera_bind_group();

            render_pass.set_pipeline(&canvas.camera_pipeline);
            render_pass.set_bind_group(0, &canvas.camera_vertex_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, canvas.camera_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                canvas.camera_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(0..canvas.index_count, 0, 0..1);
        }
    }
}
