use crate::prelude::*;
use crate::renderer::render_context::RenderContext;
use crate::resource::Resource;
use crate::texture::CRTexture;

/// Holds all GPU resources needed for canvas rendering
/// (camera pipeline for displaying the canvas texture)
pub struct CanvasContext {
    pub render_texture_a: CRTexture,
    pub render_texture_b: CRTexture,
    pub is_rendering_to_a: bool,

    // Paint pipeline resources
    pub paint_fragment_uniform: BrushFragmentUniform,
    pub paint_fragment_uniform_buffer: wgpu::Buffer,
    pub paint_uniform_bind_group: wgpu::BindGroup,
    // Paint pipeline fields
    pub paint_fragment_bind_group_a: wgpu::BindGroup,
    pub paint_fragment_bind_group_b: wgpu::BindGroup,
    pub paint_pipeline: wgpu::RenderPipeline,

    // Camera pipeline resources (for displaying canvas to screen)
    pub camera_pipeline: wgpu::RenderPipeline,
    pub camera_vertex_buffer: wgpu::Buffer,
    pub camera_index_buffer: wgpu::Buffer,
    pub camera_vertex_bind_group: wgpu::BindGroup,
    pub camera_vertex_uniform_buffer: wgpu::Buffer,
    pub camera_fragment_bind_group_a: wgpu::BindGroup,
    pub camera_fragment_bind_group_b: wgpu::BindGroup,
    pub camera_uniform: CameraUniform,

    pub index_count: u32,
}

impl CanvasContext {
    pub fn new(render_ctx: &RenderContext, window_size: (u32, u32)) -> Self {
        let device = &render_ctx.device;

        // Create ping-pong textures for painting
        let render_texture_a =
            CRTexture::create_render_texture(device, window_size, "Render Texture A (ping)");

        let render_texture_b =
            CRTexture::create_render_texture(device, window_size, "Render Texture B (pong)");

        // Camera uniform for view/projection matrix
        let mut camera_uniform = CameraUniform::new();
        let camera = Camera2D::new();
        camera_uniform.update_view_projection(&camera);

        // Fragment bind group layout (for canvas texture sampling)
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

        // Bind groups for each ping-pong texture
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

        // Vertex uniform buffer (for camera transform)
        let camera_vertex_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Vertex Uniform Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
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

        // Vertex and index buffers (full-screen quad)
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

        // Camera shader and pipeline
        let camera_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/camera.wgsl"));

        let CRRenderPipeline {
            pipeline: camera_pipeline,
            ..
        } = CRRenderPipeline::new(
            device,
            &[
                &camera_vertex_bind_group_layout,
                &camera_fragment_bind_group_layout,
            ],
            &camera_shader,
            render_ctx.config.format,
            &[DisplayVertex::desc()],
            false,
            "Camera Pipeline",
        );

        // -------------------------- //
        // ----- PAINT PIPELINE ----- //
        // -------------------------- //

        let paint_uniform =
            BrushFragmentUniform::new_with_data(DEFAULT_BRUSH_COLOR.to_rgba_array());

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

        let paint_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/paint.wgsl"));

        let CRRenderPipeline {
            pipeline: paint_pipeline,
            ..
        } = CRRenderPipeline::new(
            &device,
            &[
                &paint_uniform_bind_group_layout,
                &paint_fragment_bind_group_layout,
            ],
            &paint_shader,
            render_ctx.config.format,
            &[DisplayVertex::desc()],
            false,
            "Paint Pipeline",
        );

        // -------------------------- //
        // --- PAINT PIPELINE END --- //
        // -------------------------- //

        Self::clear_textures(
            device,
            &render_ctx.queue,
            &[&render_texture_a, &render_texture_b],
        );

        Self {
            render_texture_a,
            render_texture_b,
            is_rendering_to_a: true,
            camera_pipeline,
            camera_vertex_buffer,
            camera_index_buffer,
            camera_vertex_bind_group,
            camera_vertex_uniform_buffer,
            camera_fragment_bind_group_a,
            camera_fragment_bind_group_b,
            camera_uniform,
            index_count: INDICES.len() as u32,
            paint_pipeline,
            paint_fragment_bind_group_a,
            paint_fragment_bind_group_b,
            paint_fragment_uniform: paint_uniform,
            paint_fragment_uniform_buffer: paint_uniform_buffer,
            paint_uniform_bind_group,
        }
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

    pub fn clear_render_texture(&mut self, render_ctx: &RenderContext) {
        Self::clear_textures(
            &render_ctx.device,
            &render_ctx.queue,
            &[&self.render_texture_a, &self.render_texture_b],
        );
    }

    pub fn update_camera_buffer(&mut self, render_ctx: Res<RenderContext>, camera: &Camera2D) {
        self.camera_uniform.update_view_projection(camera);
        render_ctx.queue.write_buffer(
            &self.camera_vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn update_paint_buffer(
        &mut self,
        render_ctx: Res<RenderContext>,
        dot: &Dot2D,
        camera: &Camera2D,
    ) {
        self.paint_fragment_uniform.update_dot(dot);
        self.paint_fragment_uniform
            .update_inverse_view_projection(camera);

        render_ctx.queue.write_buffer(
            &self.paint_fragment_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.paint_fragment_uniform]),
        );
    }

    pub fn update_brush_color(&mut self, render_ctx: Res<RenderContext>, color: [f32; 4]) {
        self.paint_fragment_uniform.set_color(color);

        render_ctx.queue.write_buffer(
            &self.paint_fragment_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.paint_fragment_uniform]),
        );
    }

    /// Get the appropriate camera bind group based on ping-pong state
    pub fn get_camera_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_rendering_to_a {
            &self.camera_fragment_bind_group_a
        } else {
            &self.camera_fragment_bind_group_b
        }
    }
}

impl Resource for CanvasContext {}
