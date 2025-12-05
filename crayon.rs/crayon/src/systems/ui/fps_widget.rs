use crate::resources::frame_time::FrameTime;

pub struct FpsWidget;

impl FpsWidget {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, ctx: &egui::Context, frame_time: &FrameTime) {
        egui::TopLeftPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .inner_margin(egui::Margin::same(8.0))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                ui.heading(format!("{:05.1} FPS", frame_time.fps));
            });
    }
}
