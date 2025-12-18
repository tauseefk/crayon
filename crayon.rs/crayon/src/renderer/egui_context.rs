use std::sync::Arc;

use winit::window::Window;

use crate::renderer::render_context::RenderContext;
use crate::renderer::ui::theme::{fonts, DEFAULT_THEME};
use crate::resource::Resource;

/// egui context excapsulation that's useful for rendering the UI.
pub struct EguiContext {
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl EguiContext {
    pub fn new(window: Arc<Window>, render_context: &RenderContext) -> Self {
        let egui_ctx = egui::Context::default();

        // Install image loaders for SVG support
        egui_extras::install_image_loaders(&egui_ctx);

        // Apply theme and load fonts
        DEFAULT_THEME.apply(&egui_ctx);
        fonts::load_fonts(&egui_ctx);

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
