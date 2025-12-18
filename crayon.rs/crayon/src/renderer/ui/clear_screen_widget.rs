use egui::Vec2;

use crate::{
    app::App,
    constants::TOOLS_BG_COLOR,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::{drawable::Drawable, theme::widgets::IconButton},
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
            .fixed_pos(egui::pos2(8., height - 56.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(TOOLS_BG_COLOR)
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                let trash_icon = egui::include_image!("../../../assets/icons/trash.svg");
                if ui
                    .add(
                        IconButton::new(trash_icon)
                            .size(Vec2::splat(40.0))
                            .icon_size(Vec2::splat(20.0)),
                    )
                    .clicked()
                {
                    event_sender.send(ControllerEvent::ClearCanvas);
                }
            });
    }
}
