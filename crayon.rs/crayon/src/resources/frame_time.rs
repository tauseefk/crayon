#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use crate::resource::Resource;

const FRAME_TIME_WINDOW: usize = 10;

pub struct FrameTime {
    last_frame: Instant,
    frame_times: [Duration; FRAME_TIME_WINDOW],
    current_index: usize,
    pub fps: f32,
}

impl FrameTime {
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
            frame_times: [Duration::ZERO; FRAME_TIME_WINDOW],
            current_index: 0,
            fps: 0.0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame);
        self.last_frame = now;

        self.frame_times[self.current_index] = delta;
        self.current_index = (self.current_index + 1) % FRAME_TIME_WINDOW;

        let total: Duration = self.frame_times.iter().sum();
        let avg = total.as_secs_f32() / FRAME_TIME_WINDOW as f32;
        self.fps = if avg > 0.0 { 1.0 / avg } else { 0.0 };
    }
}

impl Resource for FrameTime {}
