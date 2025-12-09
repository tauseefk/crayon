use wgpu::SurfaceTexture;

use crate::resource::Resource;

/// Frame local resources for rendering.
pub struct FrameContext {
    pub surface_texture: Option<SurfaceTexture>,
    pub surface_view: Option<wgpu::TextureView>,
}

impl FrameContext {
    pub fn new() -> Self {
        Self {
            surface_texture: None,
            surface_view: None,
        }
    }
}

impl Resource for FrameContext {}
