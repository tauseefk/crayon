use cgmath::{EuclideanSpace, InnerSpace};

use crate::prelude::*;

pub type Dimension2D = Vector2<f32>;

/// Linearly interpolate between two `Dot2D`s.
/// Interpolates the position and the radius.
#[must_use]
pub const fn lerp_dot_2d(dot1: Dot2D, dot2: Dot2D, k: f32) -> Dot2D {
    Dot2D {
        position: Point2 {
            x: dot1.position.x + (dot2.position.x - dot1.position.x) * k,
            y: dot1.position.y + (dot2.position.y - dot1.position.y) * k,
        },
        radius: dot1.radius + (dot2.radius - dot1.radius) * k,
    }
}

pub type Rect = ([f32; 2], [f32; 2]);
#[derive(Debug, PartialEq)]
pub struct AABB {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

/// Get the center of a bounding box around a group of rects.
pub fn rects_to_center(rects: &[Rect]) -> Point2<f32> {
    let AABB { min, max } = AABB::from_rects(rects);
    Point2::new(f32::midpoint(min.x, max.x), f32::midpoint(min.y, max.y))
}

impl AABB {
    pub fn from_origin_and_size(origin: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            min: Point2::new(origin[0], origin[1]),
            max: Point2::new(origin[0] + size[0], origin[1] + size[1]),
        }
    }

    pub fn from_rects(rects: &[Rect]) -> AABB {
        let mut min = Point2::new(f32::MAX, f32::MAX);
        let mut max = Point2::new(f32::MIN, f32::MIN);
        for (position, size) in rects {
            min.x = min.x.min(position[0]);
            min.y = min.y.min(position[1]);
            max.x = max.x.max(position[0] + size[0]);
            max.y = max.y.max(position[1] + size[1]);
        }
        AABB { min, max }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// Provides the square length of a point by creating a vector from origin
#[must_use]
pub fn sqr_len(point: Point2<f32>) -> f32 {
    let v: Vector2<f32> = point - cgmath::Point2::origin();
    v.magnitude2()
}
