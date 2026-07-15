use crate::{
    app::App,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::{
        drawable::Drawable,
        theme::widgets::{GLOBAL_PADDING, PillButton},
    },
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

        egui::TopBottomPanel::top("menu")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(GLOBAL_PADDING))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add(PillButton::new("Open…")).clicked() {
                        event_sender.send(ControllerEvent::OpenDocument);
                    }
                });
            });
    }
}
