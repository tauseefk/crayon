use crate::prelude::BrushColor;

pub struct ColorPickerWidget;

impl ColorPickerWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, ctx: &egui::Context, current_color: &BrushColor) {
        egui::Window::new("Controls")
            .fixed_pos(egui::pos2(8.0, 8.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                let mut color = current_color.to_srgb();

                if ui.color_edit_button_srgb(&mut color).changed() {
                    let new_color = BrushColor::from(color);
                    println!("{new_color:?}");
                }
            });
    }
}
