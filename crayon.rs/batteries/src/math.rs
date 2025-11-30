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

/// Provides the square length of a point by creating a vector from origin
#[must_use]
pub fn sqr_len(point: Point2<f32>) -> f32 {
    let v: Vector2<f32> = point - cgmath::Point2::origin();
    v.magnitude2()
}
