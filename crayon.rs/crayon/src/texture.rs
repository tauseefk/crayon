use wgpu::{TextureFormat, TextureViewDescriptor};

#[cfg(target_arch = "wasm32")]
use crate::constants::WINDOW_SIZE;

pub struct CRTexture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl CRTexture {
    pub fn create_render_texture(
        device: &wgpu::Device,
        dimensions: (u32, u32),
        format: TextureFormat,
        label: &str,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let dimensions = (
            if dimensions.0 == 0 {
                WINDOW_SIZE.0
            } else {
                dimensions.0.min(WINDOW_SIZE.0)
            },
            if dimensions.1 == 0 {
                WINDOW_SIZE.1
            } else {
                dimensions.1.min(WINDOW_SIZE.1)
            },
        );

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some(format!("{} View", label).as_str()),
            ..wgpu::TextureViewDescriptor::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(format!("{} Sampler", label).as_str()),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}
