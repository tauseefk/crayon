use std::mem;

use cgmath::EuclideanSpace;

use crate::{
    constants::{CAMERA_ZOOM_MAX, CAMERA_ZOOM_MIN, DEFAULT_CANVAS_ZOOM, WINDOW_SIZE},
    utils::clamp,
};

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

/// This encapsulates a transformation based on pointer events.
/// Scale is relative, panning should follow the pointer.
#[derive(Default)]
pub struct CameraTransform {
    pub scale_delta: Option<f32>,
    pub translation: Option<cgmath::Point2<f32>>,
}

/// Pretend orthographic camera for rendering the entire canvas.
/// Enables the zooming and panning.
#[derive(Clone, Copy)]
pub struct Camera2D {
    /// remains the same in both axes
    scale: f32,
    viewport: (f32, f32),
    translation: cgmath::Point2<f32>,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            scale: DEFAULT_CANVAS_ZOOM,
            viewport: (WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32),
            translation: cgmath::Point2::origin(),
        }
    }

    pub fn with_viewport(width: f32, height: f32) -> Self {
        Self {
            scale: DEFAULT_CANVAS_ZOOM,
            viewport: (width, height),
            translation: cgmath::Point2::origin(),
        }
    }

    /// Updates the camera based on a transformation.
    pub fn update(&mut self, transform: &CameraTransform) {
        let CameraTransform {
            scale_delta,
            translation,
        } = transform;
        if let Some(scale_delta) = scale_delta {
            self.scale = clamp::clamp_zoom(self.scale, *scale_delta);
        }
        if let Some(translation) = translation {
            self.translation = *translation;
        }
    }

    /// Adjusts the scale to maintain constant visual zoom when window size changes.
    fn adjust_scale_for_resize(&mut self, new_width: f32) {
        let old_width = self.viewport.0;
        if old_width != new_width {
            self.scale =
                (self.scale * (old_width / new_width)).clamp(CAMERA_ZOOM_MIN, CAMERA_ZOOM_MAX);
        }
    }

    /// Updates the aspect ratio, useful when rendering a non-square canvaas.
    pub fn update_viewport(&mut self, width: f32, height: f32) {
        self.adjust_scale_for_resize(width);
        self.viewport = (width, height);
    }

    /// Builds the transformation matrix based on the scale and translation.
    /// translate -> scale
    pub fn build_2d_transform_matrix(&self) -> cgmath::Matrix4<f32> {
        let scale_matrix = cgmath::Matrix4::from_nonuniform_scale(
            self.scale,
            self.scale * self.aspect_ratio(),
            1.0,
        );

        let translation_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3::new(
            self.translation.x,
            self.translation.y,
            0.0,
        ));

        // order dependent
        translation_matrix * scale_matrix
    }

    /// Builds the inverse transformation matrix based on the scale and translation.
    /// scale -> translate
    pub fn build_2d_inverse_transform_matrix(&self) -> cgmath::Matrix4<f32> {
        let scale_matrix = cgmath::Matrix4::from_nonuniform_scale(
            1.0 / self.scale,
            1.0 / (self.scale * self.aspect_ratio()),
            1.0,
        );

        let translation_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3::new(
            -self.translation.x,
            -self.translation.y,
            0.0,
        ));

        // inverse order
        scale_matrix * translation_matrix
    }

    fn aspect_ratio(&self) -> f32 {
        self.viewport.0 / self.viewport.1
    }
}
