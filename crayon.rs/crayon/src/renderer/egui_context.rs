use std::sync::Arc;

use winit::window::Window;

use crate::renderer::render_context::RenderContext;
use crate::resource::Resource;

pub struct EguiContext {
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl EguiContext {
    pub fn new(window: Arc<Window>, render_context: &RenderContext) -> Self {
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &render_context.device,
            render_context.config.format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            egui_ctx,
            egui_state,
            egui_renderer,
        }
    }
}

impl Resource for EguiContext {}
