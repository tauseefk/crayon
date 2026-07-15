use egui::{Align2, Color32, FontId, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::renderer::ui::theme::{
    DEFAULT_THEME,
    widgets::{FONT_SIZE, PADDING},
};

/// A pill-shaped button with rounded ends. Sizes itself to its text plus
/// `PADDING` on every side, never smaller than `min_size`.
pub struct PillButton<'a> {
    text: &'a str,
    min_size: Vec2,
    padding: Vec2,
    /// `None` renders a full pill (`height / 2`).
    corner_radius: Option<f32>,
    selected: bool,
    fill: Option<Color32>,
    text_color: Option<Color32>,
}

impl<'a> PillButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            min_size: Vec2::ZERO,
            padding: Vec2::splat(PADDING),
            corner_radius: None,
            selected: false,
            fill: None,
            text_color: None,
        }
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    /// Space between the text and the button edge, per side.
    pub fn padding(mut self, padding: Vec2) -> Self {
        self.padding = padding;
        self
    }

    /// Rounded rect instead of the full pill.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Persistent selected state (the selection palette, distinct from the
    /// transient pressed state).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
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
        let font_id = FontId::new(FONT_SIZE, egui::FontFamily::Proportional);
        let galley = ui.painter().layout_no_wrap(
            self.text.to_string(),
            font_id.clone(),
            Color32::PLACEHOLDER,
        );
        let size = (galley.rect.size() + self.padding * 2.0).max(self.min_size);

        let (rect, response) = ui.allocate_exact_size(size, Sense::click());

        if ui.is_rect_visible(rect) {
            let theme = &DEFAULT_THEME;
            let painter = ui.painter();

            let (bg_color, text_color, stroke_color) = if response.is_pointer_button_down_on() {
                (
                    self.fill.unwrap_or(theme.primary),
                    self.text_color.unwrap_or(theme.on_primary),
                    theme.primary,
                )
            } else if self.selected {
                (
                    self.fill.unwrap_or(theme.primary_container),
                    self.text_color.unwrap_or(theme.on_primary_container),
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

            let rounding = self.corner_radius.unwrap_or(rect.height() / 2.0);

            painter.rect_filled(rect, rounding, bg_color);

            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0, stroke_color),
                StrokeKind::Outside,
            );

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
