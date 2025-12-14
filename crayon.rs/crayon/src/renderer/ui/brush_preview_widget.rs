use crate::{
    app::App, renderer::ui::drawable::Drawable, resource::ResourceContext,
    resources::brush_preview_state::BrushPreviewState, state::State,
};

const BRUSH_PREVIEW_SIZE_MULTIPLE: f32 = 500.0;
const MIN_BRUSH_PREVIEW_SIZE: f32 = 50.0;
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
        let center = ctx.content_rect().center();
        let camera_scale = state.camera.scale();
        let radius =
            (state.editor.brush_properties.size * BRUSH_PREVIEW_SIZE_MULTIPLE * camera_scale)
                .min(MIN_BRUSH_PREVIEW_SIZE);

        let painter = ctx.debug_painter();

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
