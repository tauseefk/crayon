use crate::{
    app::App, event_sender::EventSender, events::ControllerEvent, prelude::BrushColor,
    resource::ResourceContext, state::State, systems::ui::drawable::Drawable,
};

pub struct ColorPickerWidget;

impl ColorPickerWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for ColorPickerWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let Some(state) = app.read::<State>() else {
            return;
        };

        let Some(event_sender) = app.read::<EventSender>() else {
            return;
        };

        let current_color = &state.editor.brush_color;

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
                    event_sender.send(ControllerEvent::UpdateBrushColor(new_color));
                }
            });
    }
}
