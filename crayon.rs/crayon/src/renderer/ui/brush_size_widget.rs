use cgmath::Point2;

use crate::{
    app::App,
    constants::TOOLS_BG_COLOR,
    editor_state::BrushProperties,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::{
        brush::POINTER_TO_BRUSH_SIZE_MULTIPLE,
        ui::{drawable::Drawable, theme::widgets::StyledSlider},
    },
    resource::ResourceContext,
    resources::brush_preview_state::BrushPreviewState,
    state::State,
};

const BRUSH_STEP_SIZE: f64 = 0.5;
const MIN_BRUSH_SIZE: f32 = 5.0;
const MAX_BRUSH_SIZE: f32 = 50.0;
// y-flipped
const SCREEN_CENTER: Point2<f32> = Point2 { x: 1., y: -1. };

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
            .fixed_pos(egui::pos2(8.0, 108.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(TOOLS_BG_COLOR)
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                let mut pointer_size = state.editor.brush_properties.pointer_size;
                let response = ui.add(
                    StyledSlider::new(&mut pointer_size, MIN_BRUSH_SIZE..=MAX_BRUSH_SIZE)
                        .vertical()
                        .step_by(BRUSH_STEP_SIZE),
                );

                if response.dragged() || response.has_focus() || response.changed() {
                    // update brush preview state on user interaction
                    if let Some(mut preview_state) = app.write::<BrushPreviewState>() {
                        preview_state.show_at_position(SCREEN_CENTER);
                    }
                }

                if response.changed() {
                    event_sender.send(ControllerEvent::UpdateBrush(BrushProperties {
                        pointer_size,
                        size: pointer_size * POINTER_TO_BRUSH_SIZE_MULTIPLE,
                        ..state.editor.brush_properties
                    }));
                }
            });
    }
}
