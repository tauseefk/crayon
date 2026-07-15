use crate::{
    app::App,
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::{
        drawable::Drawable,
        theme::widgets::{GLOBAL_PADDING, IconButton},
    },
    resource::ResourceContext,
    resources::document_state::DocumentState,
};

/// Clears the selected layer; disabled when no layer is selected (the global
/// clear-canvas is retired, §3.3).
pub struct ClearScreenWidget;

impl ClearScreenWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for ClearScreenWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let (Some(event_sender), Some(doc)) =
            (app.read::<EventSender>(), app.read::<DocumentState>())
        else {
            return;
        };
        let selected_layer = doc.selection.selected_layer();

        let height = ctx.content_rect().height();

        egui::Window::new("Clear")
            .fixed_pos(egui::pos2(GLOBAL_PADDING, height - 56.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let trash_icon = egui::include_image!("../../../assets/icons/trash.svg");
                let button = ui.add_enabled(
                    selected_layer.is_some(),
                    IconButton::new(trash_icon),
                );
                if button.clicked()
                    && let Some((_, layer)) = selected_layer
                {
                    event_sender.send(ControllerEvent::ClearLayer { layer });
                }
            });
    }
}
