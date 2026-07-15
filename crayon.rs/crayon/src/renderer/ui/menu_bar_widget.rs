use crate::{
    app::App,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::drawable::Drawable,
    resource::ResourceContext,
};

/// Top menu bar (§1.9): document-level actions. The future home of
/// Save/Export.
pub struct MenuBarWidget;

impl MenuBarWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for MenuBarWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let Some(event_sender) = app.read::<EventSender>() else {
            return;
        };

        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    event_sender.send(ControllerEvent::OpenDocument);
                }
            });
        });
    }
}
