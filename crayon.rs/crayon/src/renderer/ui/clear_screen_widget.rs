use crate::{
    app::App,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::{
        drawable::Drawable,
        theme::widgets::{GLOBAL_PADDING, IconButton},
    },
    resource::ResourceContext,
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
            .fixed_pos(egui::pos2(GLOBAL_PADDING, height - 56.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let trash_icon = egui::include_image!("../../../assets/icons/trash.svg");
                if ui.add(IconButton::new(trash_icon)).clicked() {
                    event_sender.send(ControllerEvent::ClearCanvas);
                }
            });
    }
}
