use egui::{Align2, Color32, FontId, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::renderer::ui::theme::{DEFAULT_THEME, widgets::FONT_SIZE};

/// A pill-shaped button with rounded ends
#[allow(dead_code)]
pub struct PillButton<'a> {
    text: &'a str,
    min_size: Vec2,
    fill: Option<Color32>,
    text_color: Option<Color32>,
}

impl<'a> PillButton<'a> {
    #[allow(dead_code)]
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            min_size: Vec2::new(60.0, 32.0),
            fill: None,
            text_color: None,
        }
    }

    #[allow(dead_code)]
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    #[allow(dead_code)]
    pub fn fill(mut self, color: Color32) -> Self {
        self.fill = Some(color);
        self
    }

    #[allow(dead_code)]
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

            let rounding = rect.height() / 2.0;

            painter.rect_filled(rect, rounding, bg_color);

            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0, stroke_color),
                StrokeKind::Outside,
            );

            let font_id = FontId::new(FONT_SIZE, egui::FontFamily::Proportional);
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
