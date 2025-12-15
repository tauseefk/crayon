use crate::prelude::*;

pub const POINTER_SIZE: f32 = 30.0;

#[cfg(target_arch = "wasm32")]
pub const POINTER_TO_BRUSH_SIZE_MULTIPLE: f32 = 0.00334; // 1 / 300
#[cfg(not(target_arch = "wasm32"))]
pub const POINTER_TO_BRUSH_SIZE_MULTIPLE: f32 = 0.001667; // 1 / 600

pub const DEFAULT_BRUSH_SIZE: f32 = POINTER_SIZE * POINTER_TO_BRUSH_SIZE_MULTIPLE;

const BRUSH_SHARPNESS: f32 = 0.4;
const DEFAULT_BRUSH_POSITION: f32 = 2.0;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushFragmentUniform {
    color: [f32; 4],
    sharpness: f32,
    size: f32,
    position: [f32; 2],
    inverse_view_projection: [[f32; 4]; 4],
}

impl BrushFragmentUniform {
    pub fn new_with_data(color: [f32; 4]) -> Self {
        use cgmath::SquareMatrix;
        Self {
            color,
            sharpness: BRUSH_SHARPNESS,
            size: DEFAULT_BRUSH_SIZE,
            position: [DEFAULT_BRUSH_POSITION, DEFAULT_BRUSH_POSITION],
            inverse_view_projection: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    pub fn update_dot(&mut self, dot: &Dot2D) {
        self.position = [dot.position.x, dot.position.y];
        self.size = dot.radius;
    }

    pub fn update_inverse_view_projection(&mut self, camera: &Camera2D) {
        self.inverse_view_projection = camera.build_2d_inverse_transform_matrix().into();
    }
}
