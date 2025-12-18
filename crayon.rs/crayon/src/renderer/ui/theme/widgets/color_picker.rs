use egui::{Color32, Popup, Response, Sense, Stroke, Ui, Vec2, Widget, color_picker};

use crate::renderer::ui::theme::DEFAULT_THEME;

/// A circular color picker button that opens a color picker popup on click
pub struct CircularColorPicker<'a> {
    color: &'a mut [u8; 3],
    radius: f32,
    id_source: Option<&'a str>,
}

impl<'a> CircularColorPicker<'a> {
    pub fn new(color: &'a mut [u8; 3]) -> Self {
        Self {
            color,
            radius: 18.0,
            id_source: None,
        }
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn id_source(mut self, id: &'a str) -> Self {
        self.id_source = Some(id);
        self
    }
}

impl Widget for CircularColorPicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = Vec2::splat(self.radius * 2.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());

        let popup_id = ui.make_persistent_id(self.id_source.unwrap_or("color_picker_popup"));

        if ui.is_rect_visible(rect) {
            let theme = &DEFAULT_THEME;
            let painter = ui.painter();

            // Convert sRGB float to Color32
            let current_color = Color32::from_rgb(self.color[0], self.color[1], self.color[2]);

            // Determine border color based on state
            let border_color = if response.hovered() || Popup::is_id_open(ui.ctx(), popup_id) {
                theme.primary
            } else {
                theme.outline
            };

            let border_width = if response.hovered() { 2.0 } else { 1.5 };

            // Draw filled circle with current color
            painter.circle_filled(rect.center(), self.radius, current_color);

            // Draw border
            painter.circle_stroke(
                rect.center(),
                self.radius,
                Stroke::new(border_width, border_color),
            );

            // Draw inner highlight for depth
            painter.circle_stroke(
                rect.center(),
                self.radius - 2.0,
                Stroke::new(1.0, Color32::from_white_alpha(60)),
            );
        }

        // Toggle popup on click
        if response.clicked() {
            Popup::toggle_id(ui.ctx(), popup_id);
        }

        // Show color picker popup
        Popup::from_response(&response).id(popup_id).show(|ui| {
            ui.set_min_width(200.0);
            // color_picker::color_edit_button_srgb(ui, self.color);
        });

        response
    }
}
