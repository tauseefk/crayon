use crate::{
    app::App,
    constants::TOOLS_BG_COLOR,
    renderer::ui::{drawable::Drawable, theme::widgets::GLOBAL_PADDING},
    resource::ResourceContext,
    resources::frame_time::FrameTime,
};

pub struct FpsWidget;

impl FpsWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for FpsWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let frame_time = app
            .read::<FrameTime>()
            .expect("FrameTime resource not found");

        // Bottom-right of the space the panels left over: available_rect
        // already excludes the layers side panel at its live width.
        let corner =
            ctx.available_rect().right_bottom() - egui::vec2(GLOBAL_PADDING, GLOBAL_PADDING);

        egui::Window::new("FPS")
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .fixed_pos(corner)
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(TOOLS_BG_COLOR)
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                ui.heading(format!("{:05.1} FPS", frame_time.fps));
            });
    }
}
