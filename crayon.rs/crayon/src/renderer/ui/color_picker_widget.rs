use cgmath::Point2;

use crate::{
    app::App,
    constants::TOOLS_BG_COLOR,
    editor_state::{BrushColor, BrushProperties},
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::drawable::Drawable,
    resource::ResourceContext,
    resources::brush_preview_state::BrushPreviewState,
    state::State,
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

        let current_color = &state.editor.brush_properties.color;

        egui::Window::new("Color Controls")
            .fixed_pos(egui::pos2(8.0, 8.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(TOOLS_BG_COLOR)
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                let mut color = current_color.to_srgb();
                let response = ui.color_edit_button_srgb(&mut color);

                if response.changed() || response.clicked() {
                    if let Some(mut preview_state) = app.write::<BrushPreviewState>() {
                        preview_state.show_at_position(Point2 { x: 1., y: -1. });
                    }
                }

                if response.changed() {
                    let new_color = BrushColor::from(color);
                    event_sender.send(ControllerEvent::UpdateBrush(BrushProperties {
                        color: new_color,
                        ..state.editor.brush_properties
                    }));
                }
            });
    }
}
