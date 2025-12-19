use egui::{Color32, Image, ImageSource, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

use crate::renderer::ui::theme::{
    DEFAULT_THEME,
    widgets::{FONT_SIZE, ICON_SIZE, PADDING},
};

const MARGIN_RIGHT: f32 = 6.0;
const SPACING: f32 = 8.0;

/// A circular or pill-shaped button with an icon and optional text
pub struct IconButton<'a> {
    icon: ImageSource<'a>,
    text: Option<&'a str>,
    size: Vec2,
    icon_size: Vec2,
    circular: bool,
    fill: Option<Color32>,
    tint: Option<Color32>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: impl Into<ImageSource<'a>>) -> Self {
        Self {
            icon: icon.into(),
            text: None,
            size: Vec2::new(
                ICON_SIZE + PADDING * 2.0 + MARGIN_RIGHT,
                ICON_SIZE + PADDING * 2.0,
            ),
            icon_size: Vec2::splat(20.0),
            circular: true,
            fill: None,
            tint: None,
        }
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self.circular = false;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn icon_size(mut self, size: Vec2) -> Self {
        self.icon_size = size;
        self
    }

    pub fn _fill(mut self, color: Color32) -> Self {
        self.fill = Some(color);
        self
    }

    pub fn _tint(mut self, color: Color32) -> Self {
        self.tint = Some(color);
        self
    }
}

impl Widget for IconButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_at_least(
            Vec2 {
                x: self.size.x + 20.,
                y: self.size.y,
            },
            Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            let theme = &DEFAULT_THEME;
            let painter = ui.painter();

            // Determine colors based on interaction state
            let (bg_color, tint_color, stroke_color) = if response.is_pointer_button_down_on() {
                (
                    self.fill.unwrap_or(theme.primary),
                    self.tint.unwrap_or(theme.on_primary),
                    theme.primary,
                )
            } else if response.hovered() {
                (
                    self.fill.unwrap_or(theme.primary_container),
                    self.tint.unwrap_or(theme.on_primary_container),
                    theme.outline,
                )
            } else {
                (
                    self.fill.unwrap_or(theme.surface_variant),
                    self.tint.unwrap_or(theme.on_surface),
                    theme.outline_variant,
                )
            };

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

            // Draw icon and text
            if let Some(text) = self.text {
                // Calculate spacing and layout
                let font_id = egui::FontId::proportional(FONT_SIZE);

                // Calculate text width using painter
                let galley = painter.layout_no_wrap(text.to_string(), font_id.clone(), tint_color);
                let text_width = galley.rect.width();

                let content_width = self.icon_size.x + SPACING + text_width + MARGIN_RIGHT;

                // Center the content horizontally
                let content_start_x = rect.center().x - content_width / 2.0;

                // Draw icon on the left
                let icon_center =
                    egui::pos2(content_start_x + self.icon_size.x / 2.0, rect.center().y);
                let icon_rect = egui::Rect::from_center_size(icon_center, self.icon_size);
                let image = Image::new(self.icon.clone())
                    .tint(tint_color)
                    .fit_to_exact_size(self.icon_size);
                image.paint_at(ui, icon_rect);

                // Draw text on the right
                let text_pos = egui::pos2(
                    content_start_x + self.icon_size.x + SPACING,
                    rect.center().y,
                );
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    text,
                    font_id,
                    tint_color,
                );
            } else {
                // icon-only
                let icon_rect = egui::Rect::from_center_size(rect.center(), self.icon_size);
                let image = Image::new(self.icon.clone())
                    .tint(tint_color)
                    .fit_to_exact_size(self.icon_size);
                image.paint_at(ui, icon_rect);
            }
        }

        response
    }
}
