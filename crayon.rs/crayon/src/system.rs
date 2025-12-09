use crate::app::App;

#[derive(Copy, Clone)]
pub enum Schedule {
    /// Systems run once at startup.
    #[allow(dead_code)]
    Startup,
    /// Systems run at the start of each frame (setup).
    PreUpdate,
    /// Systems that go brrrrrr (main update/render).
    Update,
    /// Systems run at the end of each frame (cleanup/present).
    PostUpdate,
}

pub trait System: Send + Sync {
    fn run(&self, app: &App);
}

pub trait SystemRegistry {
    fn add_system(&mut self, schedule: Schedule, system: impl System + 'static) -> &mut Self;
}
