use crate::prelude::BrushColor;

pub struct ColorPickerWidget;

impl ColorPickerWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, ctx: &egui::Context, current_color: &BrushColor) {
        let mut color = current_color.to_srgb();

        egui::TopRightPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .inner_margin(egui::Margin::same(8.0))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                ui.color_edit_button_srgb(&mut color);
            });
    }
}
