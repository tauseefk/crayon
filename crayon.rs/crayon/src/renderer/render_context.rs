use std::sync::Arc;

use wgpu::{CommandEncoder, Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::resource::Resource;

/// Frame independent resources for rendering.
pub struct RenderContext {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub encoder: Option<CommandEncoder>,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        #[allow(unused_mut)]
        let mut size = window.inner_size();

        // window resizing events on the browser can cause problems,
        // so override with default size
        #[cfg(target_arch = "wasm32")]
        {
            use crate::prelude::WINDOW_SIZE;

            size.width = WINDOW_SIZE.0;
            size.height = WINDOW_SIZE.1;
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::defaults()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            encoder: None,
        })
    }

    pub fn reconfigure(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            self.surface.configure(&self.device, &self.config);
        }
    }
}

impl Resource for RenderContext {}
