use std::collections::VecDeque;

use crate::prelude::*;
use crate::resource::Resource;

pub struct BrushPointData {
    pub dot: Dot2D,
    pub camera: Camera2D,
}

pub struct BrushPointQueue {
    points: VecDeque<BrushPointData>,
}

impl BrushPointQueue {
    pub fn new() -> Self {
        Self {
            points: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, dot: Dot2D, camera: Camera2D) {
        self.points.push_back(BrushPointData { dot, camera });
    }

    pub fn drain(&mut self) -> impl Iterator<Item = BrushPointData> + '_ {
        self.points.drain(..)
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }
}

impl Resource for BrushPointQueue {}
