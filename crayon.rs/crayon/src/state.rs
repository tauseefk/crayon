use crate::prelude::*;

pub struct State {
    renderer_state: RendererState,
    is_surface_configured: bool,
    index_count: u32,
    pub camera: Camera2D,
    pub window: Arc<Window>,
    unified_brush_uniform: UnifiedBrushUniform,
    unified_brush_uniform_buffer: wgpu::Buffer,
    unified_brush_bind_group: wgpu::BindGroup,
    unified_world_bind_group_a: wgpu::BindGroup,
    unified_world_bind_group_b: wgpu::BindGroup,
    unified_pipeline: wgpu::RenderPipeline,
    ping_pong_toggle: bool,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let camera = Camera2D {
            scale: DEFAULT_CANVAS_ZOOM,
            translation: cgmath::Point2::origin(),
            aspect_ratio: 1.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_projection(&camera);

        let renderer_state = RendererState::new(window.clone(), camera_uniform).await?;

        Self::clear_texture(
            &renderer_state.context.device,
            &renderer_state.context.queue,
            &renderer_state.render_texture_a,
        );

        Self::clear_texture(
            &renderer_state.context.device,
            &renderer_state.context.queue,
            &renderer_state.render_texture_b,
        );

        // -------------------------- //
        // ------ UNIFIED PASS ------ //
        // -------------------------- //

        let unified_brush_uniform = UnifiedBrushUniform::new();

        let unified_brush_uniform_buffer =
            renderer_state
                .context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Unified Brush Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[unified_brush_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let unified_brush_bind_group_layout = renderer_state
            .context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("Unified Brush Bind Group Layout"),
            });

        let unified_brush_bind_group =
            renderer_state
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &unified_brush_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: unified_brush_uniform_buffer.as_entire_binding(),
                    }],
                    label: Some("Unified Brush Bind Group"),
                });

        let unified_world_bind_group_layout = renderer_state
            .context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: Some("Unified World Bind Group Layout"),
            });

        let unified_world_bind_group_a =
            renderer_state
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &unified_world_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &renderer_state.render_texture_a.view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                &renderer_state.render_texture_a.sampler,
                            ),
                        },
                    ],
                    label: Some("Unified World Bind Group A"),
                });

        let unified_world_bind_group_b =
            renderer_state
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &unified_world_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &renderer_state.render_texture_b.view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                &renderer_state.render_texture_b.sampler,
                            ),
                        },
                    ],
                    label: Some("Unified World Bind Group B"),
                });

        let unified_shader = renderer_state
            .context
            .device
            .create_shader_module(wgpu::include_wgsl!("renderer/shaders/unified.wgsl"));

        let CRRenderPipeline {
            pipeline: unified_pipeline,
            ..
        } = CRRenderPipeline::new(
            &renderer_state.context.device,
            &[
                &unified_brush_bind_group_layout,
                &unified_world_bind_group_layout,
            ],
            unified_shader,
            renderer_state.config.format,
            &[DisplayVertex::desc()],
            false,
            "Unified Pipeline",
        )?;

        // -------------------------- //
        // ---- UNIFIED PASS END ---- //
        // -------------------------- //

        Ok(Self {
            renderer_state,
            is_surface_configured: false,
            index_count: INDICES.len() as u32,
            camera,
            unified_brush_uniform,
            unified_brush_uniform_buffer,
            unified_brush_bind_group,
            unified_world_bind_group_a,
            unified_world_bind_group_b,
            unified_pipeline,
            ping_pong_toggle: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.renderer_state.config.width = width;
            self.renderer_state.config.height = height;
            self.renderer_state.surface.configure(
                &self.renderer_state.context.device,
                &self.renderer_state.config,
            );
            self.is_surface_configured = true;

            self.camera.update_aspect_ratio(width as f32, height as f32);
            self.update_display();
        }
    }

    fn clear_texture(device: &wgpu::Device, queue: &wgpu::Queue, render_texture: &CRTexture) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Render Texture Encoder"),
        });

        {
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
        Self::clear_texture(
            &self.renderer_state.context.device,
            &self.renderer_state.context.queue,
            &self.renderer_state.render_texture_a,
        );
    }

    pub fn update_display(&mut self) {
        self.renderer_state
            .camera_uniform
            .update_view_projection(&self.camera);
        self.renderer_state.context.queue.write_buffer(
            &self.renderer_state.display_vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.renderer_state.camera_uniform.clone()]),
        );
    }

    pub fn update_unified_brush(&mut self, position: Point2<f32>) {
        self.unified_brush_uniform.update_position(position);
        self.unified_brush_uniform
            .update_inverse_view_projection(&self.camera);

        self.renderer_state.context.queue.write_buffer(
            &self.unified_brush_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.unified_brush_uniform]),
        );
    }

    pub fn accumulate_brush_to_texture(&mut self) {
        if !self.is_surface_configured {
            return;
        }

        let (read_bind_group, write_texture_view) = if self.ping_pong_toggle {
            (
                &self.unified_world_bind_group_a,
                &self.renderer_state.render_texture_b.view,
            )
        } else {
            (
                &self.unified_world_bind_group_b,
                &self.renderer_state.render_texture_a.view,
            )
        };

        let mut encoder = self.renderer_state.context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Accumulate Brush Encoder"),
            },
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Accumulate Brush Pass"),
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

            render_pass.set_pipeline(&self.unified_pipeline);
            render_pass.set_bind_group(0, &self.unified_brush_bind_group, &[]);
            render_pass.set_bind_group(1, read_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.renderer_state.display_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.renderer_state.display_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.renderer_state
            .context
            .queue
            .submit(std::iter::once(encoder.finish()));

        self.ping_pong_toggle = !self.ping_pong_toggle;
    }

    pub fn unified_render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.renderer_state.surface.get_current_texture()?;

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let display_bind_group = if self.ping_pong_toggle {
            &self.renderer_state.display_fragment_bind_group_a
        } else {
            &self.renderer_state.display_fragment_bind_group_b
        };

        let mut encoder = self.renderer_state.context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Display Encoder"),
            },
        );

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

            render_pass.set_pipeline(&self.renderer_state.display_pipeline);
            render_pass.set_bind_group(0, &self.renderer_state.display_vertex_bind_group, &[]);
            render_pass.set_bind_group(1, display_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.renderer_state.display_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.renderer_state.display_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.renderer_state
            .context
            .queue
            .submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        output.present();

        Ok(())
    }
}
