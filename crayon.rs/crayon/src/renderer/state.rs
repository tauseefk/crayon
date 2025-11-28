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
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub context: RendererContext,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub index_count: u32,
    pub camera_uniform: CameraUniform,
    pub camera_vertex_buffer: wgpu::Buffer,
    pub camera_vertex_bind_group: wgpu::BindGroup,
    pub camera_vertex_uniform_buffer: wgpu::Buffer,
    /// bind group for `ping` shader
    pub camera_fragment_bind_group_a: wgpu::BindGroup,
    /// bind group for `pong` shader
    pub camera_fragment_bind_group_b: wgpu::BindGroup,
    pub camera_index_buffer: wgpu::Buffer,
    pub camera_pipeline: wgpu::RenderPipeline,
    /// ping texture
    pub render_texture_a: CRTexture,
    /// pong texture
    pub render_texture_b: CRTexture,
    // Paint pipeline fields
    pub paint_fragment_uniform: BrushFragmentUniform,
    pub paint_fragment_uniform_buffer: wgpu::Buffer,
    pub paint_uniform_bind_group: wgpu::BindGroup,
    pub paint_fragment_bind_group_a: wgpu::BindGroup,
    pub paint_fragment_bind_group_b: wgpu::BindGroup,
    pub paint_pipeline: wgpu::RenderPipeline,
    /// `true` if rendering to a, `false` if rendering to b
    is_rendering_to_a: bool,
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

        let camera_fragment_bind_group_layout =
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
                label: Some("Camera Fragment Bind Group Layout"),
            });

        let camera_fragment_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_fragment_bind_group_layout,
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
            label: Some("Camera Fragment Bind Group A"),
        });

        let camera_fragment_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_fragment_bind_group_layout,
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
            label: Some("Camera Fragment Bind Group B"),
        });

        // -------------------------- //
        // ------ FRAGMENT END ------ //
        // -------------------------- //

        // -------------------------- //
        // --------- VERTEX --------- //
        // -------------------------- //

        let camera_vertex_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Vertex Uniform Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform.clone()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_vertex_bind_group_layout =
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
                label: Some("Camera Vertex Bind Group Layout"),
            });

        let camera_vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_vertex_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_vertex_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Camera Vertex Bind Group"),
        });

        let camera_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Vertex Buffer"),
            contents: bytemuck::cast_slice(DISPLAY_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let camera_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/camera.wgsl"));

        let CRRenderPipeline {
            pipeline: camera_pipeline,
            ..
        } = CRRenderPipeline::new(
            &device,
            &[
                &camera_vertex_bind_group_layout,
                &camera_fragment_bind_group_layout,
            ],
            camera_shader,
            config.format,
            &[DisplayVertex::desc()],
            false,
            "Camera Pipeline",
        )?;
        // -------------------------- //
        // ------- VERTEX END ------- //
        // -------------------------- //

        // -------------------------- //
        // ----- PAINT PIPELINE ----- //
        // -------------------------- //

        let paint_uniform = BrushFragmentUniform::new();

        let paint_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paint Uniform Buffer"),
            contents: bytemuck::cast_slice(&[paint_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let paint_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Paint Bind Group Layout"),
            });

        let paint_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &paint_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: paint_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Paint Bind Group"),
        });

        let paint_fragment_bind_group_layout =
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
                label: Some("Paint Bind Group Layout B"),
            });

        let paint_fragment_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &paint_fragment_bind_group_layout,
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
            label: Some("Paint Bind Group A"),
        });

        let paint_fragment_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &paint_fragment_bind_group_layout,
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
            label: Some("Paint Bind Group B"),
        });

        let paint_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/paint.wgsl"));

        let CRRenderPipeline {
            pipeline: paint_pipeline,
            ..
        } = CRRenderPipeline::new(
            &device,
            &[
                &paint_uniform_bind_group_layout,
                &paint_fragment_bind_group_layout,
            ],
            paint_shader,
            config.format,
            &[DisplayVertex::desc()],
            false,
            "Paint Pipeline",
        )?;

        // -------------------------- //
        // --- PAINT PIPELINE END --- //
        // -------------------------- //

        Ok(Self {
            window,
            surface,
            context: RendererContext {
                _adapter: adapter,
                device,
                _instance: instance,
                queue,
            },
            config,
            is_surface_configured: false,
            index_count: INDICES.len() as u32,
            camera_uniform,
            camera_vertex_bind_group,
            camera_vertex_buffer,
            camera_vertex_uniform_buffer,
            camera_index_buffer,
            camera_fragment_bind_group_a,
            camera_fragment_bind_group_b,
            camera_pipeline,
            render_texture_a,
            render_texture_b,
            paint_fragment_uniform: paint_uniform,
            paint_fragment_uniform_buffer: paint_uniform_buffer,
            paint_uniform_bind_group,
            paint_fragment_bind_group_a,
            paint_fragment_bind_group_b,
            paint_pipeline,
            is_rendering_to_a: true,
        })
    }

    pub fn configure(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.context.device, &self.config);
        self.is_surface_configured = true;
    }

    fn clear_textures(device: &wgpu::Device, queue: &wgpu::Queue, render_textures: &[&CRTexture]) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Render Texture Encoder"),
        });

        for render_texture in render_textures {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn clear_render_texture(&mut self) {
        Self::clear_textures(
            &self.context.device,
            &self.context.queue,
            &[&self.render_texture_a, &self.render_texture_b],
        );
    }

    pub fn update_camera_buffer(&mut self, camera: &Camera2D) {
        self.camera_uniform.update_view_projection(camera);
        self.context.queue.write_buffer(
            &self.camera_vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform.clone()]),
        );
    }

    pub fn update_paint_buffer(&mut self, position: Point2<f32>, camera: &Camera2D) {
        self.paint_fragment_uniform.update_position(position);
        self.paint_fragment_uniform
            .update_inverse_view_projection(camera);

        self.context.queue.write_buffer(
            &self.paint_fragment_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.paint_fragment_uniform]),
        );
    }

    pub fn paint_to_texture(&mut self) {
        if !self.is_surface_configured {
            return;
        }

        let (read_bind_group, write_texture_view) = if self.is_rendering_to_a {
            (
                &self.paint_fragment_bind_group_b,
                &self.render_texture_a.view,
            )
        } else {
            (
                &self.paint_fragment_bind_group_a,
                &self.render_texture_b.view,
            )
        };

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Paint Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Paint Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: write_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.paint_pipeline);
            // dynamic offsets are not necessary,
            // as we're passing single chunks of data to the GPU per bind group
            render_pass.set_bind_group(0, &self.paint_uniform_bind_group, &[]);
            render_pass.set_bind_group(1, read_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.camera_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.camera_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));

        self.is_rendering_to_a = !self.is_rendering_to_a;
    }

    pub fn get_camera_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_rendering_to_a {
            &self.camera_fragment_bind_group_a
        } else {
            &self.camera_fragment_bind_group_b
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let camera_bind_group = self.get_camera_bind_group();

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Display Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Display Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.camera_pipeline);
            render_pass.set_bind_group(0, &self.camera_vertex_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.camera_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.camera_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        output.present();

        Ok(())
    }
}
