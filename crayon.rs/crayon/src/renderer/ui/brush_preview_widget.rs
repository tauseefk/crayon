use batteries::prelude::world_to_screen_position;
use egui::Pos2;

use crate::{
    app::App, renderer::ui::drawable::Drawable, resource::ResourceContext,
    resources::brush_preview_state::BrushPreviewState, state::State,
};

const PREVIEW_CIRCLE_STROKE: f32 = 2.0;

pub struct BrushPreviewWidget;

impl BrushPreviewWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for BrushPreviewWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let (Some(state), Some(preview_state)) =
            (app.read::<State>(), app.read::<BrushPreviewState>())
        else {
            return;
        };

        if !preview_state.is_visible() {
            return;
        }

        let color = &state.editor.brush_properties.color;
        let ui_scale = preview_state.scale();
        let radius = state.editor.brush_properties.pointer_size * ui_scale;

        let painter = ctx.debug_painter();

        let position = preview_state.position();
        let window_size = ctx.content_rect().size();
        let position = world_to_screen_position(position, (window_size.x, window_size.y));
        let center = Pos2 {
            x: position.x,
            y: position.y,
        };
        painter.circle_filled(center, radius, color.to_egui_color());

        painter.circle_stroke(
            center,
            radius,
            egui::Stroke::new(PREVIEW_CIRCLE_STROKE, egui::Color32::WHITE),
        );
        painter.circle_stroke(
            center,
            radius + PREVIEW_CIRCLE_STROKE,
            egui::Stroke::new(PREVIEW_CIRCLE_STROKE, egui::Color32::BLACK),
        );
    }
}
