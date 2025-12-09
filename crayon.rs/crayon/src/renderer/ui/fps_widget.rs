use crate::{
    app::App, renderer::ui::drawable::Drawable, resource::ResourceContext,
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

        let screen_width = ctx.content_rect().width();

        egui::Window::new("FPS")
            .fixed_pos(egui::pos2(screen_width - 80.0 - 20.0, 8.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                ui.heading(format!("{:05.1} FPS", frame_time.fps));
            });
    }
}
