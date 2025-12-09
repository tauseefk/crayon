use std::sync::Arc;

use wgpu::{CommandEncoder, Device, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

use crate::resource::Resource;

pub struct RenderContext {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub encoder: Option<CommandEncoder>,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> Self {
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

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

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

        #[cfg(target_arch = "wasm32")]
        log::info!("RenderContext surface config: {}x{}", config.width, config.height);

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            encoder: None,
        }
    }

    pub fn reconfigure(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        #[cfg(target_arch = "wasm32")]
        log::info!("reconfigure called with size: {:?}", new_size);

        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            #[cfg(target_arch = "wasm32")]
            log::info!("reconfigure setting surface to: {}x{}", self.config.width, self.config.height);

            self.surface.configure(&self.device, &self.config);
        }
    }
}

impl Resource for RenderContext {}
