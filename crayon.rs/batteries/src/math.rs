use crate::prelude::*;

pub type Dimension2D = Vector2<f32>;

pub fn clamp(t: f32, min: f32, max: f32) -> f32 {
    t.clamp(min, max)
}

pub fn smooth_step(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0., 1.);

    t * t * (3.0 - 2.0 * t)
}

///  Converts screen space coordinates to normalized WGPU coordinates (-1, 1)
///
pub fn normalize(point: Point2<f32>, range: Dimension2D) -> Point2<f32> {
    Point2 {
        x: (point.x / range.x) * 2. - 1.,
        y: (point.y / range.y) * 2. - 1.,
    }
}

pub fn lerp_dot_2d(dot1: Dot2D, dot2: Dot2D, k: f32) -> Dot2D {
    Dot2D {
        position: Point2 {
            x: dot1.position.x + (dot2.position.x - dot1.position.x) * k,
            y: dot1.position.y + (dot2.position.y - dot1.position.y) * k,
        },
        radius: dot1.radius + (dot2.radius - dot1.radius) * k,
    }
}

/// Provides the length of a point
/// Honestly this should prob not exist and I should just convert the point to a vec and get the length
pub fn sqr_len(point: Point2<f32>) -> f32 {
    point.mul_element_wise(point).sum()
}
