use crate::app::{App, WindowResource};
use crate::renderer::egui_context::EguiContext;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::render_context::RenderContext;
use crate::renderer::ui::color_picker_widget::ColorPickerWidget;
use crate::renderer::ui::drawable::Drawable;
use crate::renderer::ui::fps_widget::FpsWidget;
use crate::resource::ResourceContext;
use crate::system::System;

/// Renders Tools UI
pub struct ToolsSystem {
    tools: [Box<dyn Drawable>; 2],
}

impl ToolsSystem {
    pub fn new() -> Self {
        Self {
            tools: [
                Box::new(FpsWidget::new()),
                Box::new(ColorPickerWidget::new()),
            ],
        }
    }
}

impl System for ToolsSystem {
    fn run(&self, app: &App) {
        let Some(mut egui_ctx_res) = app.write::<EguiContext>() else {
            return;
        };
        let Some(mut render_ctx_res) = app.write::<RenderContext>() else {
            return;
        };
        let Some(frame_ctx_res) = app.read::<FrameContext>() else {
            return;
        };
        let Some(window_res) = app.write::<WindowResource>() else {
            return;
        };

        let raw_input = egui_ctx_res.egui_state.take_egui_input(&window_res.0);

        let full_output = egui_ctx_res.egui_ctx.run(raw_input, |ctx| {
            for tool in &self.tools {
                tool.draw(ctx, app);
            }
        });

        egui_ctx_res
            .egui_state
            .handle_platform_output(&window_res.0, full_output.platform_output);

        let tris = egui_ctx_res
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let size = window_res.0.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            egui_ctx_res.egui_renderer.update_texture(
                &render_ctx_res.device,
                &render_ctx_res.queue,
                *id,
                image_delta,
            );
        }

        let mut encoder =
            render_ctx_res
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui encoder"),
                });

        egui_ctx_res.egui_renderer.update_buffers(
            &render_ctx_res.device,
            &render_ctx_res.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        render_ctx_res
            .queue
            .submit(std::iter::once(encoder.finish()));

        let Some(view) = frame_ctx_res.surface_view.as_ref() else {
            return;
        };
        let Some(frame_encoder) = render_ctx_res.encoder.as_mut() else {
            return;
        };

        {
            let render_pass = frame_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // draw on top of existing content
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            egui_ctx_res.egui_renderer.render(
                &mut render_pass.forget_lifetime(),
                &tris,
                &screen_descriptor,
            );
        }

        for id in &full_output.textures_delta.free {
            egui_ctx_res.egui_renderer.free_texture(id);
        }
    }
}
