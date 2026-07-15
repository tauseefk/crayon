use batteries::prelude::Dot2D;

use crate::{renderer::camera::Camera2D, resource::Resource};

const BRUSH_POINT_QUEUE_SIZE: usize = 500;
/// Fields are read again by the paint stage (S3); until then queued points
/// are drained and dropped.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct BrushPointData {
    pub dot: Dot2D,
    pub camera: Camera2D,
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

    pub fn write(&mut self, dot: Dot2D, camera: Camera2D) {
        self.points.write(BrushPointData { dot, camera });
    }

    pub fn read(&mut self) -> Option<BrushPointData> {
        self.points.read()
    }
}

impl Resource for BrushPointQueue {}
