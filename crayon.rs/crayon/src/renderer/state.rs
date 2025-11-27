use crate::prelude::*;

pub struct RendererContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub _instance: wgpu::Instance,
    pub _adapter: wgpu::Adapter,
}

/// This lives on the main thread as surface creation cannot take place elsewhere
///
/// Consists of all the data required to update and render the artboard.
///
/// It receives a texture and renders it to the screen.
pub struct RendererState {
    pub surface: wgpu::Surface<'static>,
    pub context: RendererContext,
    pub config: wgpu::SurfaceConfiguration,
    pub camera_uniform: CameraUniform,
    pub display_vertex_buffer: wgpu::Buffer,
    pub display_vertex_bind_group: wgpu::BindGroup,
    pub display_vertex_uniform_buffer: wgpu::Buffer,
    /// bind group for `ping` shader
    pub display_fragment_bind_group_a: wgpu::BindGroup,
    /// bind group for `pong` shader
    pub display_fragment_bind_group_b: wgpu::BindGroup,
    pub display_index_buffer: wgpu::Buffer,
    pub display_pipeline: wgpu::RenderPipeline,
    /// ping texture
    pub render_texture_a: CRTexture,
    /// pong texture
    pub render_texture_b: CRTexture,
}

impl RendererState {
    /// take ownership of parameters as relevant ones are re-exported later
    pub async fn new(window: Arc<Window>, camera_uniform: CameraUniform) -> anyhow::Result<Self> {
        // mut for wasm32
        #[allow(unused_mut)]
        let mut size = window.inner_size();

        // window resizing events on the browser can cause problems,
        // so override with default size
        #[cfg(target_arch = "wasm32")]
        {
            size.width = WINDOW_SIZE.0;
            size.height = WINDOW_SIZE.1;
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::defaults()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // -------------------------- //
        // -------- FRAGMENT -------- //
        // -------------------------- //

        let render_texture_a = CRTexture::create_render_texture(
            &device,
            window.inner_size().into(),
            "Render Texture A (ping)",
        );

        let render_texture_b = CRTexture::create_render_texture(
            &device,
            window.inner_size().into(),
            "Render Texture B (pong)",
        );

        let display_fragment_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Display Fragment Bind Group Layout"),
            });

        let display_fragment_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &display_fragment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_texture_a.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_texture_a.sampler),
                },
            ],
            label: Some("Display Fragment Bind Group"),
        });

        let display_fragment_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &display_fragment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_texture_b.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_texture_b.sampler),
                },
            ],
            label: Some("Display Fragment Bind Group B"),
        });

        // -------------------------- //
        // ------ FRAGMENT END ------ //
        // -------------------------- //

        // -------------------------- //
        // --------- VERTEX --------- //
        // -------------------------- //

        let display_vertex_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Display Vertex Uniform Buffer (Camera)"),
                contents: bytemuck::cast_slice(&[camera_uniform.clone()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let display_vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Display Vertex Bind Group Layout"),
            });

        let display_vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &display_vertex_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: display_vertex_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Display Vertex Bind Group"),
        });

        let display_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Display Vertex Buffer"),
            contents: bytemuck::cast_slice(DISPLAY_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let display_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Display Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let display_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/display.wgsl"));

        let CRRenderPipeline {
            pipeline: display_pipeline,
            ..
        } = CRRenderPipeline::new(
            &device,
            &[
                &display_vertex_bind_group_layout,
                &display_fragment_bind_group_layout,
            ],
            display_shader,
            config.format,
            &[DisplayVertex::desc()],
            false,
            "Display Pipeline",
        )?;
        // -------------------------- //
        // ------- VERTEX END ------- //
        // -------------------------- //

        Ok(Self {
            surface,
            context: RendererContext {
                _adapter: adapter,
                device,
                _instance: instance,
                queue,
            },
            config,
            camera_uniform,
            display_vertex_bind_group,
            display_vertex_buffer,
            display_vertex_uniform_buffer,
            display_index_buffer,
            display_fragment_bind_group_a,
            display_fragment_bind_group_b,
            display_pipeline,
            render_texture_a,
            render_texture_b,
        })
    }
}
