use cgmath::{Point2, Transform};

use crate::prelude::Camera2D;

pub fn transform_point(point: Point2<f32>, camera: &Camera2D) -> Point2<f32> {
    let transform_matrix = camera.build_2d_transform_matrix();
    let point_3d = cgmath::Point3::new(point.x, point.y, 0.0);
    let transformed = transform_matrix.transform_point(point_3d);
    Point2 {
        x: transformed.x,
        y: transformed.y,
    }
}
