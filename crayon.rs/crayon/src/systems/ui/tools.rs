use crate::app::{App, WindowResource};
use crate::renderer::egui_context::EguiContext;
use crate::renderer::render_context::RenderContext;
use crate::resource::ResourceContext;
use crate::resources::frame_time::FrameTime;
use crate::state::State;
use crate::system::System;

use super::{ColorPickerWidget, FpsWidget};

pub struct ToolsSystem {
    fps_widget: FpsWidget,
    color_picker_widget: ColorPickerWidget,
}

impl ToolsSystem {
    pub fn new() -> Self {
        Self {
            fps_widget: FpsWidget::new(),
            color_picker_widget: ColorPickerWidget::new(),
        }
    }
}

impl System for ToolsSystem {
    fn run(&self, app: &App) {
        let frame_time = app
            .read::<FrameTime>()
            .expect("FrameTime resource not found");

        // Early return if State is not initialized yet
        let Some(state_res) = app.read::<State>() else {
            return;
        };

        let mut egui_ctx_res = app
            .write::<EguiContext>()
            .expect("EguiContext resource not found");
        let render_ctx_res = app
            .read::<RenderContext>()
            .expect("RenderContext resource not found");
        let window_res = app
            .read::<WindowResource>()
            .expect("WindowResource resource not found");

        // once per frame
        let output = render_ctx_res
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture");

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = egui_ctx_res.egui_state.take_egui_input(&window_res.0);

        let full_output = egui_ctx_res.egui_ctx.run(raw_input, |ctx| {
            self.fps_widget.draw(ctx, &frame_time);
            self.color_picker_widget
                .draw(ctx, &state_res.editor.brush_color);
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

        let mut encoder =
            render_ctx_res
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("UI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
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

        // once per frame
        render_ctx_res
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        for id in &full_output.textures_delta.free {
            egui_ctx_res.egui_renderer.free_texture(id);
        }
    }
}
