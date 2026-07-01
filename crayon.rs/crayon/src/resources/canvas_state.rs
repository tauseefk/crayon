use wgpu::util::DeviceExt;

use crate::editor_state::DEFAULT_BRUSH_COLOR;
use crate::renderer::camera::{Camera2D, CameraUniform, DISPLAY_VERTICES, DisplayVertex};
use crate::renderer::pipeline::CRRenderPipeline;
use crate::renderer::render_context::RenderContext;
use crate::resource::Resource;
use crate::texture::CRTexture;

const INDICES: &[u16] = &[
    0, 1, 2, // bottom right triangle
    0, 2, 3, // left top triangle
];

/// Upper bound on dabs stamped in a single frame. The brush point queue is capped at
/// 500, so a frame never drains more than that.
const MAX_DABS_PER_FRAME: usize = 1024;

/// Per-instance data for the accumulate pass: `xy` is the dab center in canvas NDC,
/// `z` is the radius in NDC.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DabInstance {
    pub center: [f32; 2],
    pub radius: f32,
}

impl DabInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DabUniform {
    color: [f32; 4],
}

pub struct CanvasContext {
    pub render_texture_a: CRTexture,
    pub render_texture_b: CRTexture,
    pub is_rendering_to_a: bool,

    stroke_texture: CRTexture,
    pub stroke_layer_bind_group: wgpu::BindGroup,

    dab_uniform: DabUniform,
    dab_uniform_buffer: wgpu::Buffer,
    dab_uniform_bind_group: wgpu::BindGroup,
    dab_instance_buffer: wgpu::Buffer,
    // Reused CPU staging for the frame's dabs; capacity is reserved to avoid reallocation.
    dab_scratch: Vec<DabInstance>,
    accumulate_pipeline: wgpu::RenderPipeline,

    // Composites `stroke_layer` over the canvas. Used for on-screen display
    // and for the stroke-end merge (with an identity transform).
    pub camera_pipeline: wgpu::RenderPipeline,
    pub camera_vertex_buffer: wgpu::Buffer,
    pub camera_index_buffer: wgpu::Buffer,
    pub camera_vertex_bind_group: wgpu::BindGroup,
    pub camera_vertex_uniform_buffer: wgpu::Buffer,
    identity_camera_bind_group: wgpu::BindGroup,
    camera_fragment_bind_group_a: wgpu::BindGroup,
    camera_fragment_bind_group_b: wgpu::BindGroup,
    camera_uniform: CameraUniform,

    pub index_count: u32,
}

impl CanvasContext {
    pub fn new(render_ctx: &RenderContext, window_size: (u32, u32)) -> Self {
        let device = &render_ctx.device;
        let format = render_ctx.config.format;

        let render_texture_a = CRTexture::create_render_texture(
            device,
            window_size,
            format,
            "Render Texture A (ping)",
        );
        let render_texture_b = CRTexture::create_render_texture(
            device,
            window_size,
            format,
            "Render Texture B (pong)",
        );
        let stroke_layer =
            CRTexture::create_render_texture(device, window_size, format, "Stroke Layer");

        let mut camera_uniform = CameraUniform::new();
        let camera = Camera2D::new();
        camera_uniform.update_view_projection(&camera);

        // Texture + sampler layout, shared by the canvas and the stroke layer.
        let texture_bind_group_layout =
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
                label: Some("Texture Bind Group Layout"),
            });

        let texture_bind_group = |texture: &CRTexture, label: &str| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some(label),
            })
        };

        let camera_fragment_bind_group_a =
            texture_bind_group(&render_texture_a, "Canvas Bind Group A");
        let camera_fragment_bind_group_b =
            texture_bind_group(&render_texture_b, "Canvas Bind Group B");
        let stroke_layer_bind_group = texture_bind_group(&stroke_layer, "Stroke Layer Bind Group");

        // Vertex-stage camera uniform (view-projection), plus an identity copy for merges.
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

        let camera_vertex_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Vertex Uniform Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_vertex_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_vertex_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Camera Vertex Bind Group"),
        });

        let identity_camera_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Identity Camera Uniform Buffer"),
                contents: bytemuck::cast_slice(&[CameraUniform::new()]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let identity_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_vertex_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: identity_camera_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Identity Camera Bind Group"),
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

        let camera_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/camera.wgsl"));

        let CRRenderPipeline {
            pipeline: camera_pipeline,
            ..
        } = CRRenderPipeline::new(
            device,
            &[
                &camera_vertex_bind_group_layout,
                &texture_bind_group_layout,
                &texture_bind_group_layout,
            ],
            &camera_shader,
            format,
            &[DisplayVertex::desc()],
            None,
            "Camera Pipeline",
        );

        // --------------------------- //
        // ----- ACCUMULATE PASS ----- //
        // --------------------------- //

        let dab_uniform = DabUniform {
            color: DEFAULT_BRUSH_COLOR.to_rgba_array(),
        };

        let dab_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dab Uniform Buffer"),
            contents: bytemuck::cast_slice(&[dab_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let dab_uniform_bind_group_layout =
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
                label: Some("Dab Uniform Bind Group Layout"),
            });

        let dab_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &dab_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: dab_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Dab Uniform Bind Group"),
        });

        let dab_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dab Instance Buffer"),
            size: (MAX_DABS_PER_FRAME * std::mem::size_of::<DabInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // native backends use srgb textures, so the brush color is linearized in-shader
        #[cfg(not(target_arch = "wasm32"))]
        let dab_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/dab_linear.wgsl"));

        #[cfg(target_arch = "wasm32")]
        let dab_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/dab.wgsl"));

        let CRRenderPipeline {
            pipeline: accumulate_pipeline,
            ..
        } = CRRenderPipeline::new(
            device,
            &[&dab_uniform_bind_group_layout],
            &dab_shader,
            format,
            &[DabInstance::desc()],
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            "Accumulate Pipeline",
        );

        // --------------------------- //
        // --- ACCUMULATE PASS END --- //
        // --------------------------- //

        Self::clear_textures(
            device,
            &render_ctx.queue,
            &[&render_texture_a, &render_texture_b],
            wgpu::Color::WHITE,
        );
        Self::clear_textures(
            device,
            &render_ctx.queue,
            &[&stroke_layer],
            wgpu::Color::TRANSPARENT,
        );

        Self {
            render_texture_a,
            render_texture_b,
            is_rendering_to_a: true,
            stroke_texture: stroke_layer,
            stroke_layer_bind_group,
            dab_uniform,
            dab_uniform_buffer,
            dab_uniform_bind_group,
            dab_instance_buffer,
            dab_scratch: Vec::with_capacity(MAX_DABS_PER_FRAME),
            accumulate_pipeline,
            camera_pipeline,
            camera_vertex_buffer,
            camera_index_buffer,
            camera_vertex_bind_group,
            camera_vertex_uniform_buffer,
            identity_camera_bind_group,
            camera_fragment_bind_group_a,
            camera_fragment_bind_group_b,
            camera_uniform,
            index_count: INDICES.len() as u32,
        }
    }

    fn clear_textures(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_textures: &[&CRTexture],
        color: wgpu::Color,
    ) {
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
                        load: wgpu::LoadOp::Clear(color),
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
            wgpu::Color::WHITE,
        );
        Self::clear_textures(
            &render_ctx.device,
            &render_ctx.queue,
            &[&self.stroke_texture],
            wgpu::Color::TRANSPARENT,
        );
    }

    pub fn update_camera_buffer(&mut self, render_ctx: &RenderContext, camera: &Camera2D) {
        self.camera_uniform.update_view_projection(camera);
        render_ctx.queue.write_buffer(
            &self.camera_vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn update_brush(&mut self, render_ctx: &RenderContext, color: [f32; 4]) {
        self.dab_uniform.color = color;
        render_ctx.queue.write_buffer(
            &self.dab_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.dab_uniform]),
        );
    }

    /// Clears the reusable dab staging buffer
    pub fn begin_dabs(&mut self) -> &mut Vec<DabInstance> {
        self.dab_scratch.clear();
        &mut self.dab_scratch
    }

    /// Uploads the staged dabs into the instance buffer
    pub fn upload_dabs(&self, render_ctx: &RenderContext) -> u32 {
        let count = self.dab_scratch.len().min(MAX_DABS_PER_FRAME);
        if count == 0 {
            return 0;
        }
        render_ctx.queue.write_buffer(
            &self.dab_instance_buffer,
            0,
            bytemuck::cast_slice(&self.dab_scratch[..count]),
        );
        u32::try_from(count).unwrap_or(0)
    }

    /// Stamps the frame's dabs into the stroke layer in a single pass. `clear` resets the
    /// layer first (start of a new stroke).
    pub fn accumulate_stroke(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        clear: bool,
        instance_count: u32,
    ) {
        let load = if clear {
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
        } else {
            wgpu::LoadOp::Load
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Accumulate Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.stroke_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        if instance_count > 0 {
            pass.set_pipeline(&self.accumulate_pipeline);
            pass.set_bind_group(0, &self.dab_uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.dab_instance_buffer.slice(..));
            pass.draw(0..6, 0..instance_count);
        }
    }

    pub fn record_merge_and_clear(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let (read_canvas, write_view) = if self.is_rendering_to_a {
            (
                &self.camera_fragment_bind_group_a,
                &self.render_texture_b.view,
            )
        } else {
            (
                &self.camera_fragment_bind_group_b,
                &self.render_texture_a.view,
            )
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Merge Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: write_view,
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

            pass.set_pipeline(&self.camera_pipeline);
            pass.set_bind_group(0, &self.identity_camera_bind_group, &[]);
            pass.set_bind_group(1, read_canvas, &[]);
            pass.set_bind_group(2, &self.stroke_layer_bind_group, &[]);
            pass.set_vertex_buffer(0, self.camera_vertex_buffer.slice(..));
            pass.set_index_buffer(
                self.camera_index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.is_rendering_to_a = !self.is_rendering_to_a;

        self.clear_stroke_texture(encoder);
    }

    fn clear_stroke_texture(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Stroke Layer Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.stroke_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    /// Get the appropriate canvas bind group based on ping-pong state
    pub fn get_camera_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_rendering_to_a {
            &self.camera_fragment_bind_group_a
        } else {
            &self.camera_fragment_bind_group_b
        }
    }
}

impl Resource for CanvasContext {}
