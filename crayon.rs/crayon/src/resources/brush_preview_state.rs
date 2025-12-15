#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use crate::resource::Resource;

const PREVIEW_TIMEOUT_MS: u64 = 500;

pub struct BrushPreviewState {
    pub visible: bool,
    last_interaction: Option<Instant>,
    timeout_duration: Duration,
}

impl BrushPreviewState {
    pub fn new() -> Self {
        Self {
            visible: false,
            last_interaction: None,
            timeout_duration: Duration::from_millis(PREVIEW_TIMEOUT_MS),
        }
    }

    /// Should run on user interaction
    pub fn mark_interaction(&mut self) {
        self.last_interaction = Some(Instant::now());
        self.visible = true;
    }

    /// Should run every frame to toggle preview
    pub fn update(&mut self) {
        if let Some(last_time) = self.last_interaction {
            let elapsed = Instant::now().duration_since(last_time);
            if elapsed >= self.timeout_duration {
                self.visible = false;
            }
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Resource for BrushPreviewState {}
