use crate::{
    app::App, event_sender::EventSender, events::ControllerEvent, prelude::TOOLS_BG_COLOR,
    renderer::ui::drawable::Drawable, resource::ResourceContext,
};

pub struct ClearScreenWidget;

impl ClearScreenWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for ClearScreenWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let Some(event_sender) = app.read::<EventSender>() else {
            return;
        };

        let height = ctx.content_rect().height();

        egui::Window::new("Clear")
            .fixed_pos(egui::pos2(8., height - 40.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(TOOLS_BG_COLOR)
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                if ui.add_sized([40.0, 20.0], egui::Button::new("🗑")).clicked() {
                    event_sender.send(ControllerEvent::ClearCanvas);
                }
            });
    }
}
