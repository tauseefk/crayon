mod egui_renderer;

use crate::prelude::*;
use egui_renderer::UiRenderer;
use egui_wgpu::ScreenDescriptor;
use egui_wgpu::wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use winit::event::WindowEvent;
use winit::window::Window;

pub struct CrayonUI {
    renderer: UiRenderer,
    event_sender: EventSender,
}

impl CrayonUI {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        window: &Window,
        event_sender: EventSender,
    ) -> Self {
        let renderer = UiRenderer::new(device, surface_format, None, 1, window);

        Self {
            renderer,
            event_sender,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        // Pass event to egui and trust its consumption logic
        self.renderer.handle_input(window, event)
    }

    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        surface_view: &TextureView,
        current_brush_color: BrushColor,
    ) {
        let window_size = window.inner_size();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window_size.width, window_size.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        self.renderer.begin_frame(window);

        egui::Window::new("Controls")
            .fixed_pos(egui::pos2(8.0, 8.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&self.renderer.context().style())
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(self.renderer.context(), |ui| {
                let egui_color = current_brush_color.to_egui_color32();

                // Calculate text color for readability
                let text_color = if egui_color == COLOR_A.to_egui_color32() {
                    egui::Color32::from_rgb(0xED, 0xED, 0xED)
                } else {
                    egui::Color32::from_rgb(0x2F, 0x2F, 0x2F)
                };

                let button =
                    egui::Button::new(egui::RichText::new("Toggle Color").color(text_color))
                        .fill(egui_color)
                        .stroke(egui::Stroke::NONE)
                        .min_size(egui::vec2(120.0, 40.0));

                if ui.add(button).clicked() {
                    // Toggle between COLOR_A and COLOR_B
                    let next_color = if current_brush_color == COLOR_A {
                        COLOR_B
                    } else {
                        COLOR_A
                    };
                    self.event_sender
                        .send(ControllerEvent::UpdateBrushColor(next_color));
                }
            });

        self.renderer.end_frame_and_draw(
            device,
            queue,
            encoder,
            window,
            surface_view,
            screen_descriptor,
        );
    }
}
