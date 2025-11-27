use crate::prelude::*;

const BRUSH_SIZE: f32 = 40.0 * 0.001667;
const BRUSH_SHARPNESS: f32 = 0.5;
const DEFAULT_BRUSH_POSITION: f32 = 2.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushVertex {
    coords: [f32; 2],
}

impl BrushVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// y-axis is flipped
/// wgpu world coordinates have y-axis pointing up
/// but texture coordinates have y-axis point down
#[rustfmt::skip]
pub const BRUSH_VERTICES: &[BrushVertex] = &[
    // left bottom
    BrushVertex { coords: [-1.0, -1.0] },
    // right bottom
    BrushVertex { coords: [1.0, -1.0] },
    // right top
    BrushVertex { coords: [1.0, 1.0] },
    // left top
    BrushVertex { coords: [-1.0, 1.0] },
];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushVertexUniform {
    // can't use cgmath with bytemuck directly
    // so convert all attributes into arrays
    position: [f32; 3],
    size: f32,
    inverse_view_projection: [[f32; 4]; 4],
}

impl BrushVertexUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            position: [DEFAULT_BRUSH_POSITION, DEFAULT_BRUSH_POSITION, 0.0],
            size: BRUSH_SIZE,
            inverse_view_projection: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_inverse_view_projection(&mut self, camera: &Camera2D) {
        self.inverse_view_projection = camera.build_2d_inverse_transform_matrix().into();
    }

    pub fn update_position(&mut self, position: cgmath::Point2<f32>) {
        self.position = [position.x, position.y, 0.0];
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrushFragmentUniform {
    // can't use cgmath with bytemuck directly
    // so convert all attributes into arrays
    color: [f32; 4],
    sharpness: f32,
    _padding: [f32; 3],
}

impl BrushFragmentUniform {
    pub fn new() -> Self {
        Self {
            color: [
                DEFAULT_BRUSH_COLOR.r as f32,
                DEFAULT_BRUSH_COLOR.g as f32,
                DEFAULT_BRUSH_COLOR.b as f32,
                DEFAULT_BRUSH_COLOR.a as f32,
            ],
            sharpness: BRUSH_SHARPNESS,
            _padding: [0.0; 3],
        }
    }
}
