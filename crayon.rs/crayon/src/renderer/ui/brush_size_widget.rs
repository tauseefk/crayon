use crate::{
    app::App, event_sender::EventSender, events::ControllerEvent, prelude::BrushProperties,
    renderer::ui::drawable::Drawable, resource::ResourceContext,
    resources::brush_preview_state::BrushPreviewState, state::State,
};

const BRUSH_STEP_SIZE: f64 = 0.001;
const MIN_BRUSH_SIZE: f32 = 0.005;
const MAX_BRUSH_SIZE: f32 = 0.2;

pub struct BrushSizeWidget;

impl BrushSizeWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for BrushSizeWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let (Some(state), Some(event_sender)) = (app.read::<State>(), app.read::<EventSender>())
        else {
            return;
        };

        egui::Window::new("Size Controls")
            .fixed_pos(egui::pos2(8.0, 84.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                let mut size = state.editor.brush_properties.size;
                let response = ui.add(
                    egui::Slider::new(&mut size, MIN_BRUSH_SIZE..=MAX_BRUSH_SIZE)
                        .step_by(BRUSH_STEP_SIZE)
                        .show_value(false)
                        .vertical(),
                );

                if response.dragged() || response.has_focus() || response.changed() {
                    // update brush preview state on user interaction
                    if let Some(mut preview_state) = app.write::<BrushPreviewState>() {
                        preview_state.mark_interaction();
                    }
                }

                if response.changed() {
                    event_sender.send(ControllerEvent::UpdateBrush(BrushProperties {
                        size,
                        ..state.editor.brush_properties
                    }));
                }
            });
    }
}
