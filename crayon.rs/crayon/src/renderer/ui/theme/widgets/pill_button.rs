use egui::{Align2, Color32, FontId, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::renderer::ui::theme::DEFAULT_THEME;

/// A pill-shaped button with rounded ends
pub struct PillButton<'a> {
    text: &'a str,
    min_size: Vec2,
    fill: Option<Color32>,
    text_color: Option<Color32>,
}

impl<'a> PillButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            min_size: Vec2::new(60.0, 32.0),
            fill: None,
            text_color: None,
        }
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    pub fn fill(mut self, color: Color32) -> Self {
        self.fill = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color32) -> Self {
        self.text_color = Some(color);
        self
    }
}

impl Widget for PillButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.min_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let theme = &DEFAULT_THEME;
            let painter = ui.painter();

            // Determine colors based on interaction state
            let (bg_color, text_color, stroke_color) = if response.is_pointer_button_down_on() {
                (
                    self.fill.unwrap_or(theme.primary),
                    self.text_color.unwrap_or(theme.on_primary),
                    theme.primary,
                )
            } else if response.hovered() {
                (
                    self.fill.unwrap_or(theme.primary_container),
                    self.text_color.unwrap_or(theme.on_primary_container),
                    theme.outline,
                )
            } else {
                (
                    self.fill.unwrap_or(theme.surface_variant),
                    self.text_color.unwrap_or(theme.on_surface),
                    theme.outline_variant,
                )
            };

            // Full pill rounding (half of height)
            let rounding = rect.height() / 2.0;

            // Draw background
            painter.rect_filled(rect, rounding, bg_color);

            // Draw border
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0, stroke_color),
                StrokeKind::Outside,
            );

            // Draw text centered
            let font_id = FontId::new(14.0, egui::FontFamily::Proportional);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.text,
                font_id,
                text_color,
            );
        }

        response
    }
}
