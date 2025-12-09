use crate::app::App;
use crate::resource::ResourceContext;
use crate::resources::frame_time::FrameTime;
use crate::system::System;

pub struct FrameTimeUpdateSystem;

impl System for FrameTimeUpdateSystem {
    fn run(&self, app: &App) {
        if let Some(mut frame_time) = app.write::<FrameTime>() {
            frame_time.update();
        }
    }
}
