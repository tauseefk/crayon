use cgmath::{EuclideanSpace, Point2};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use crate::constants::DEFAULT_CANVAS_ZOOM;
use crate::resource::Resource;
use crate::utils::clamp;

const PREVIEW_TIMEOUT_MS: u64 = 500;

pub struct BrushPreviewState {
    pub visible: bool,
    position: Point2<f32>,
    last_interaction: Option<Instant>,
    timeout_duration: Duration,
    scale: f32,
}

impl BrushPreviewState {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: Point2::origin(),
            last_interaction: None,
            timeout_duration: Duration::from_millis(PREVIEW_TIMEOUT_MS),
            scale: DEFAULT_CANVAS_ZOOM,
        }
    }

    /// Should run on user interaction
    pub fn mark_interaction(&mut self) {
        self.last_interaction = Some(Instant::now());
        self.visible = true;
    }

    /// Updates brush preview position, toggles visibility
    pub fn show_at_position(&mut self, position: Point2<f32>) {
        self.position = position;
        self.mark_interaction();
    }

    pub fn position(&self) -> Point2<f32> {
        self.position
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

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn update_scale(&mut self, scale_delta: f32) {
        self.scale = clamp::clamp_zoom(self.scale, scale_delta);
    }
}

impl Resource for BrushPreviewState {}
