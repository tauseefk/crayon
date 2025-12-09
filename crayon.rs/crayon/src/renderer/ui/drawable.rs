use crate::app::App;

pub trait Drawable: Send + Sync {
    fn draw(&self, ctx: &egui::Context, app: &App);
}
