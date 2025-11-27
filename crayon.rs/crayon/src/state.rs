use crate::prelude::*;

pub struct State {
    renderer_state: RendererState,
    is_surface_configured: bool,
    index_count: u32,
    pub camera: Camera2D,
    pub brush_vertex_uniform: BrushVertexUniform,
    brush_fragment_uniform: BrushFragmentUniform,
    brush_vertex_buffer: wgpu::Buffer,
    brush_vertex_uniform_buffer: wgpu::Buffer,
    brush_fragment_uniform_buffer: wgpu::Buffer,
    brush_bind_group: wgpu::BindGroup,
    brush_pipeline: wgpu::RenderPipeline,
    pub window: Arc<Window>,
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

        // -------------------------- //
        // ---------- BRUSH --------- //
        // -------------------------- //

        let mut brush_vertex_uniform = BrushVertexUniform::new();
        brush_vertex_uniform.update_inverse_view_projection(&camera);

        let brush_vertex_uniform_buffer =
            renderer_state
                .context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Brush Vertex Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[brush_vertex_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // TODO: updates this in the update method when brush attributes change
        let brush_fragment_uniform = BrushFragmentUniform::new();

        let brush_fragment_uniform_buffer =
            renderer_state
                .context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Brush Fragment Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[brush_fragment_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // TODO: is a single bind group appropriate here?
        // even though we have two separate buffers for vertex and fragment
        let brush_bind_group_layout = renderer_state.context.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Brush Bind Group Layout"),
            },
        );

        let brush_bind_group =
            renderer_state
                .context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &brush_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: brush_vertex_uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: brush_fragment_uniform_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("brush_bind_group"),
                });

        let brush_vertex_buffer =
            renderer_state
                .context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Brush Vertex Buffer"),
                    contents: bytemuck::cast_slice(BRUSH_VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let brush_shader = renderer_state
            .context
            .device
            .create_shader_module(wgpu::include_wgsl!("renderer/shaders/brush.wgsl"));

        let CRRenderPipeline {
            pipeline: brush_pipeline,
            ..
        } = CRRenderPipeline::new(
            &renderer_state.context.device,
            &[&brush_bind_group_layout],
            brush_shader,
            renderer_state.config.format,
            &[BrushVertex::desc()],
            true,
            "Brush Pipeline",
        )?;
        // -------------------------- //
        // -------- BRUSH END ------- //
        // -------------------------- //

        Self::clear_texture(
            &renderer_state.context.device,
            &renderer_state.context.queue,
            &renderer_state.render_texture,
        );

        Ok(Self {
            renderer_state,
            is_surface_configured: false,
            index_count: INDICES.len() as u32,
            camera,
            brush_vertex_uniform,
            brush_fragment_uniform,
            brush_vertex_buffer,
            brush_vertex_uniform_buffer,
            brush_fragment_uniform_buffer,
            brush_bind_group,
            brush_pipeline,
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

    fn clear_texture(device: &wgpu::Device, queue: &wgpu::Queue, render_texture: &AWTexture) {
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
            &self.renderer_state.render_texture,
        );
    }

    pub fn update_brush(&mut self, position: Point2<f32>) {
        self.brush_vertex_uniform.update_position(position);
        self.brush_vertex_uniform
            .update_inverse_view_projection(&self.camera);

        self.renderer_state.context.queue.write_buffer(
            &self.brush_vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.brush_vertex_uniform]),
        );
        self.renderer_state.context.queue.write_buffer(
            &self.brush_fragment_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.brush_fragment_uniform]),
        );
    }

    pub fn render_to_world_texture(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let mut encoder = self.renderer_state.context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Brush Render Encoder"),
            },
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Brush Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer_state.render_texture.view,
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

            render_pass.set_pipeline(&self.brush_pipeline);
            render_pass.set_bind_group(0, &self.brush_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.brush_vertex_buffer.slice(..));
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

        Ok(())
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

    pub fn display_world_texture(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.renderer_state.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.renderer_state.context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            },
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            render_pass.set_vertex_buffer(0, self.renderer_state.display_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.renderer_state.display_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.set_bind_group(1, &self.renderer_state.display_fragment_bind_group, &[]);
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
