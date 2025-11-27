use crate::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DisplayVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl DisplayVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

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
pub const DISPLAY_VERTICES: &[DisplayVertex] = &[
    // left bottom
    DisplayVertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
    // right bottom
    DisplayVertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    },
    // right top
    DisplayVertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    // left top
    DisplayVertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // can't use cgmath with bytemuck directly
    // so convert the Matrix4 into a 4x4 f32 array
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_projection: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_projection(&mut self, camera: &Camera2D) {
        self.view_projection = camera.build_2d_transform_matrix().into();
    }
}

pub struct Camera2D {
    /// remains the same in both axes
    pub scale: f32,
    pub translation: cgmath::Point2<f32>,
    pub aspect_ratio: f32,
}

impl Camera2D {
    pub fn update_aspect_ratio(&mut self, width: f32, height: f32) {
        self.aspect_ratio = width / height;
    }

    pub fn build_2d_transform_matrix(&self) -> cgmath::Matrix4<f32> {
        let scale_matrix = cgmath::Matrix4::from_nonuniform_scale(
            self.scale, 
            self.scale * self.aspect_ratio, 
            1.0
        );

        let translation_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3::new(
            self.translation.x,
            self.translation.y,
            0.0,
        ));

        // order dependent
        translation_matrix * scale_matrix
    }

    pub fn build_2d_inverse_transform_matrix(&self) -> cgmath::Matrix4<f32> {
        let scale_matrix = cgmath::Matrix4::from_nonuniform_scale(
            1.0 / self.scale, 
            1.0 / (self.scale * self.aspect_ratio), 
            1.0
        );

        let translation_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3::new(
            -self.translation.x,
            -self.translation.y,
            0.0,
        ));

        // inverse order
        scale_matrix * translation_matrix
    }
}
