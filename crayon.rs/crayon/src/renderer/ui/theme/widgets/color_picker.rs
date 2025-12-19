use egui::{
    Color32, Frame, Popup, Response, Sense, Ui, Vec2, Widget,
    color_picker::{self, Alpha},
};

use crate::constants::TOOLS_BG_COLOR;

const COLOR_PICKER_WIDTH: f32 = 220.0;

/// A circular color picker button that opens a color picker popup on click
pub struct CircularColorPicker<'a> {
    color: &'a mut Color32,
    radius: f32,
    id_source: Option<&'a str>,
}

impl<'a> CircularColorPicker<'a> {
    pub fn new(color: &'a mut Color32) -> Self {
        Self {
            color,
            radius: 16.0,
            id_source: None,
        }
    }

    #[allow(dead_code)]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    #[allow(dead_code)]
    pub fn id_source(mut self, id: &'a str) -> Self {
        self.id_source = Some(id);
        self
    }
}

impl Widget for CircularColorPicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = Vec2::splat(self.radius * 2.0);
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());

        let popup_id = ui.make_persistent_id(self.id_source.unwrap_or("color_picker_popup"));

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            let current_color = Color32::from_rgb(self.color[0], self.color[1], self.color[2]);
            // Darken color on hover
            let current_color = if response.hovered() {
                Color32::from_rgb(
                    (current_color[0] as f32 * 0.8) as u8,
                    (current_color[1] as f32 * 0.8) as u8,
                    (current_color[2] as f32 * 0.8) as u8,
                )
            } else {
                current_color
            };
            painter.circle_filled(rect.center(), self.radius, current_color);
        }

        Popup::from_toggle_button_response(&response)
            .frame(Frame::window(ui.style()).fill(TOOLS_BG_COLOR))
            .id(popup_id)
            .show(|ui| {
                ui.set_min_width(COLOR_PICKER_WIDTH);
                ui.spacing_mut().slider_width = COLOR_PICKER_WIDTH;

                let is_color_changed =
                    color_picker::color_picker_color32(ui, self.color, Alpha::Opaque);

                if is_color_changed {
                    response.mark_changed();
                }
            });

        response
    }
}
