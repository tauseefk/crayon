use batteries::prelude::Dot2D;

use crate::{
    renderer::camera::Camera2D, resource::Resource, resources::stroke_state::StrokeTarget,
};

const BRUSH_POINT_QUEUE_SIZE: usize = 500;
#[derive(Clone, Copy)]
pub struct BrushPointData {
    pub dot: Dot2D,
    pub camera: Camera2D,
    pub target: Option<StrokeTarget>,
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

    pub fn write(&mut self, brush_point_data: BrushPointData) {
        self.points.write(brush_point_data);
    }

    pub fn read(&mut self) -> Option<BrushPointData> {
        self.points.read()
    }
}

impl Resource for BrushPointQueue {}
