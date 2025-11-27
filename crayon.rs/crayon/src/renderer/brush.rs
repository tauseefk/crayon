use crate::prelude::*;

const BRUSH_SIZE: f32 = 40.0 * 0.001667;
const BRUSH_SHARPNESS: f32 = 0.2;
const DEFAULT_BRUSH_POSITION: f32 = 2.0;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UnifiedBrushUniform {
    color: [f32; 4],
    sharpness: f32,
    size: f32,
    position: [f32; 2],
    inverse_view_projection: [[f32; 4]; 4],
}

impl UnifiedBrushUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            color: [
                DEFAULT_BRUSH_COLOR.r as f32,
                DEFAULT_BRUSH_COLOR.g as f32,
                DEFAULT_BRUSH_COLOR.b as f32,
                DEFAULT_BRUSH_COLOR.a as f32,
            ],
            sharpness: BRUSH_SHARPNESS,
            size: BRUSH_SIZE,
            position: [DEFAULT_BRUSH_POSITION, DEFAULT_BRUSH_POSITION],
            inverse_view_projection: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_position(&mut self, position: cgmath::Point2<f32>) {
        self.position = [position.x, position.y];
    }

    pub fn update_inverse_view_projection(&mut self, camera: &Camera2D) {
        self.inverse_view_projection = camera.build_2d_inverse_transform_matrix().into();
    }
}
