use batteries::prelude::Dot2D;

use crate::{
    document::{ArtboardId, LayerId},
    renderer::camera::Camera2D,
    resource::Resource,
};

const BRUSH_POINT_QUEUE_SIZE: usize = 500;

/// One queued brush point: raw screen-px position, the camera as it was when
/// the point arrived, and the stroke target captured at enqueue time — a
/// selection change mid-flight cannot retarget queued points.
#[derive(Clone, Copy)]
pub struct BrushPointData {
    pub dot: Dot2D,
    pub camera: Camera2D,
    pub target: Option<(ArtboardId, LayerId)>,
}

pub struct BrushPointQueue {
    points: rasengan::Rasengan<BrushPointData, BRUSH_POINT_QUEUE_SIZE>,
}

impl BrushPointQueue {
    pub fn new() -> Self {
        Self {
            points: rasengan::Rasengan::new(),
        }
    }

    pub fn write(
        &mut self,
        dot: Dot2D,
        camera: Camera2D,
        target: Option<(ArtboardId, LayerId)>,
    ) {
        self.points.write(BrushPointData {
            dot,
            camera,
            target,
        });
    }

    pub fn read(&mut self) -> Option<BrushPointData> {
        self.points.read()
    }
}

impl Resource for BrushPointQueue {}
