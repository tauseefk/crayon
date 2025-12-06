use crate::app::App;

#[derive(Copy, Clone)]
pub enum Schedule {
    /// Systems run once at startup.
    #[allow(dead_code)]
    Startup,
    /// Systems that go brrrrrr.
    Update,
}

pub trait System: Send + Sync {
    fn run(&self, app: &App);
}

pub trait SystemRegistry {
    fn add_system(&mut self, schedule: Schedule, system: impl System + 'static) -> &mut Self;
}
