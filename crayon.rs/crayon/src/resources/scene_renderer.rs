use std::collections::HashMap;

use batteries::prelude::AABB;
use cgmath::Point2;
use wgpu::util::DeviceExt;

use crate::{
    constants::{CLEAR_COLOR, WHITE},
    document::{Document, LayerId, loader::LoadedDocument},
    editor_state::DEFAULT_BRUSH_COLOR,
    renderer::{
        camera::{Camera2D, CameraUniform},
        pipeline::CRRenderPipeline,
    },
    resource::Resource,
    resources::stroke_state::StrokeTarget,
    texture::CRTexture,
};

type RectLeft = u32;
type RectTop = u32;
type RectWidth = u32;
type RectHeight = u32;

/// Initial slot count of the quad instance buffer.
const INITIAL_QUAD_CAPACITY: usize = 256;

/// Upper bound on points stamped in a single frame.
/// The brush point queue is capped at 500, so a frame never drains more than that.
const MAX_POINTS_PER_FRAME: usize = 1024;

/// The two fixed quads of the merge pass: layer content, then stroke on top.
const MERGE_QUAD_COUNT: usize = 2;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointInstance {
    /// layer clip space
    // TODO: convert this to world space for consistency and transform in the shaders
    pub center: [f32; 2],
    /// World space
    pub radius_px: f32,
}

impl PointInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Matches `PointUniform` in `point.wgsl` / `point_linear.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PointUniform {
    color: [f32; 4],
    layer_size: [f32; 2],
    _padding: [f32; 2],
}

/// Per-instance data for the quad compositor.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    /// World top-left position.
    pub origin: [f32; 2],
    pub size: [f32; 2],
    /// uv min.xy, max.xy, up to the scratch texture size.
    pub uv_rect: [f32; 4],
}

impl QuadInstance {
    pub const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1=> Float32x2, 2 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Texture bind group sampled during the scene pass.
enum QuadBinding {
    /// 1x1 opaque white artboard background.
    Background,
    Layer(LayerId),
    /// In-progress stroke.
    Stroke,
}

/// An artboard's contiguous collection of quads and scissor rect.
struct ArtboardBatch {
    scissor: ScissorRect,
    start_idx: u32,
    count: u32,
}

pub struct LayerGpuResources {
    pub texture: CRTexture,
    pub bind_group: wgpu::BindGroup,
    pub size: (u32, u32),
}

/// GPU side of the document
///
/// Holds handles to one artboard-sized texture per layer and quad compositor that draws everything visible in the viewport.
pub struct SceneRenderer {
    quad_pipeline: wgpu::RenderPipeline,
    quad_instance_buffer: wgpu::Buffer,
    quad_capacity: usize,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    _background_texture: CRTexture,
    background_bind_group: wgpu::BindGroup,
    pub layers: HashMap<LayerId, LayerGpuResources>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,

    // Reused per-frame
    quad_scratch: Vec<QuadInstance>,
    binding_scratch: Vec<QuadBinding>,
    batch_scratch: Vec<ArtboardBatch>,

    // point accumulation
    accumulate_pipeline: wgpu::RenderPipeline,
    point_uniform: PointUniform,
    point_uniform_buffer: wgpu::Buffer,
    point_uniform_bind_group: wgpu::BindGroup,
    point_instance_buffer: wgpu::Buffer,
    point_scratch: Vec<PointInstance>,

    // shared scratch, sized to max artboard dims
    stroke_scratch: CRTexture,
    stroke_bind_group: wgpu::BindGroup,
    merge_scratch: CRTexture,
    scratch_size: (u32, u32),

    // merge pass
    merge_camera_uniform: CameraUniform,
    merge_camera_buffer: wgpu::Buffer,
    merge_camera_bind_group: wgpu::BindGroup,
    merge_quad_buffer: wgpu::Buffer,
}

impl SceneRenderer {
    /// Isomorphic renderer, can be used to render to window, or to offscreen buffer.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
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

        let camera_bind_group_layout =
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
                label: Some("Camera Bind Group Layout"),
            });

        let camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        let white_texture = CRTexture::create_render_texture(device, (1, 1), format, "Background");
        queue.write_texture(
            white_texture.texture.as_image_copy(),
            &WHITE,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let background_bind_group = Self::texture_bind_group(
            device,
            &texture_bind_group_layout,
            &white_texture,
            "Background",
        );

        let quad_instance_buffer = Self::create_quad_buffer(device, INITIAL_QUAD_CAPACITY);

        let quad_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/quad.wgsl"));
        let CRRenderPipeline {
            pipeline: quad_pipeline,
            ..
        } = CRRenderPipeline::new(
            device,
            &[&camera_bind_group_layout, &texture_bind_group_layout],
            &quad_shader,
            format,
            &[QuadInstance::desc()],
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            "Quad Pipeline",
        );

        let point_uniform = PointUniform {
            color: DEFAULT_BRUSH_COLOR.to_rgba_array(),
            layer_size: [1.0, 1.0],
            _padding: [0.0, 0.0],
        };
        let point_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Uniform Buffer"),
            contents: bytemuck::cast_slice(&[point_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let point_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // the vertex stage reads layer_size, the fragment stage color
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Point Uniform Bind Group Layout"),
            });
        let point_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &point_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: point_uniform_buffer.as_entire_binding(),
            }],
            label: Some("Point Uniform Bind Group"),
        });

        let point_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Point Instance Buffer"),
            size: (MAX_POINTS_PER_FRAME * std::mem::size_of::<PointInstance>())
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // native backends use srgb textures, so the brush color is linearized in-shader
        #[cfg(not(target_arch = "wasm32"))]
        let point_shader = device
            .create_shader_module(wgpu::include_wgsl!("../renderer/shaders/point_linear.wgsl"));
        #[cfg(target_arch = "wasm32")]
        let point_shader =
            device.create_shader_module(wgpu::include_wgsl!("../renderer/shaders/point.wgsl"));

        let CRRenderPipeline {
            pipeline: accumulate_pipeline,
            ..
        } = CRRenderPipeline::new(
            device,
            &[&point_uniform_bind_group_layout],
            &point_shader,
            format,
            &[PointInstance::desc()],
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            "Accumulate Pipeline",
        );

        // shared scratch and merge pass resources

        let stroke_scratch =
            CRTexture::create_render_texture(device, (1, 1), format, "Stroke Scratch");
        let stroke_bind_group = Self::texture_bind_group(
            device,
            &texture_bind_group_layout,
            &stroke_scratch,
            "Stroke Scratch",
        );
        let merge_scratch =
            CRTexture::create_render_texture(device, (1, 1), format, "Merge Scratch");

        let merge_camera_uniform = CameraUniform::new();
        let merge_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Merge Camera Uniform Buffer"),
            contents: bytemuck::cast_slice(&[merge_camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let merge_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: merge_camera_buffer.as_entire_binding(),
            }],
            label: Some("Scratch Ortho Bind Group"),
        });
        let merge_quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Merge Quad Buffer"),
            size: (MERGE_QUAD_COUNT * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            quad_pipeline,
            quad_instance_buffer,
            quad_capacity: INITIAL_QUAD_CAPACITY,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            _background_texture: white_texture,
            background_bind_group,
            layers: HashMap::new(),
            texture_bind_group_layout,
            format,
            quad_scratch: Vec::new(),
            binding_scratch: Vec::new(),
            batch_scratch: Vec::new(),
            accumulate_pipeline,
            point_uniform,
            point_uniform_buffer,
            point_uniform_bind_group,
            point_instance_buffer,
            point_scratch: Vec::with_capacity(MAX_POINTS_PER_FRAME),
            stroke_scratch,
            stroke_bind_group,
            merge_scratch,
            scratch_size: (1, 1),
            merge_camera_uniform,
            merge_camera_buffer,
            merge_camera_bind_group,
            merge_quad_buffer,
        }
    }

    pub fn hydrate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, loaded: &LoadedDocument) {
        self.layers.clear();
        let mut max_size = (1, 1);
        for artboard in &loaded.document.artboards {
            let size = artboard.pixel_size();
            max_size = (max_size.0.max(size.0), max_size.1.max(size.1));
            for layer in &artboard.layers {
                self.create_layer_resources(device, layer.id, size);
                if let Some(pixels) = loaded.layer_pixels.get(&layer.id) {
                    let layer_gpu = &self.layers[&layer.id];
                    queue.write_texture(
                        layer_gpu.texture.texture.as_image_copy(),
                        pixels,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * size.0),
                            rows_per_image: None,
                        },
                        wgpu::Extent3d {
                            width: size.0,
                            height: size.1,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        }
        self.ensure_scratch(device, max_size);
    }

    pub fn ensure_scratch(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.scratch_size.0 >= size.0 && self.scratch_size.1 >= size.1 {
            return;
        }

        let size = (
            size.0.max(self.scratch_size.0),
            size.1.max(self.scratch_size.1),
        );
        self.stroke_scratch =
            CRTexture::create_render_texture(device, size, self.format, "Stroke Scratch");
        self.stroke_bind_group = Self::texture_bind_group(
            device,
            &self.texture_bind_group_layout,
            &self.stroke_scratch,
            "Stroke Scratch",
        );
        self.merge_scratch =
            CRTexture::create_render_texture(device, size, self.format, "Merge Scratch");
        self.scratch_size = size;
    }

    pub fn create_layer_resources(&mut self, device: &wgpu::Device, id: LayerId, size: (u32, u32)) {
        let texture =
            CRTexture::create_render_texture(device, size, self.format, &format!("Layer {}", id.0));
        let bind_group = Self::texture_bind_group(
            device,
            &self.texture_bind_group_layout,
            &texture,
            &format!("Layer {}", id.0),
        );
        self.layers.insert(
            id,
            LayerGpuResources {
                texture,
                bind_group,
                size,
            },
        );
    }

    /// Clears the reusable point staging buffer.
    pub fn begin_points(&mut self) -> &mut Vec<PointInstance> {
        self.point_scratch.clear();
        &mut self.point_scratch
    }

    /// Uploads the staged dabs into the instance buffer.
    pub fn upload_points(&self, queue: &wgpu::Queue) -> u32 {
        let count = self.point_scratch.len().min(MAX_POINTS_PER_FRAME);
        if count == 0 {
            return 0;
        }
        queue.write_buffer(
            &self.point_instance_buffer,
            0,
            bytemuck::cast_slice(&self.point_scratch[..count]),
        );
        u32::try_from(count).unwrap_or(0)
    }

    pub fn update_brush(&mut self, queue: &wgpu::Queue, color: [f32; 4]) {
        self.point_uniform.color = color;
        self.write_point_uniform(queue);
    }

    /// Stamps the frame's points into the stroke scratch in a single pass,
    /// The viewport is confined to the active layer's size so scratch texels map 1:1 to layer texels.
    /// `clear` resets the scratch.
    pub fn accumulate_stroke(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        clear: bool,
        instance_count: u32,
        layer_size: (u32, u32),
    ) {
        #[allow(clippy::cast_precision_loss)]
        let layer_size = [layer_size.0 as f32, layer_size.1 as f32];
        self.point_uniform.layer_size = layer_size;
        self.write_point_uniform(queue);

        let load = if clear {
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
        } else {
            wgpu::LoadOp::Load
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Accumulate Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.stroke_scratch.view,
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
            pass.set_viewport(0.0, 0.0, layer_size[0], layer_size[1], 0.0, 1.0);
            pass.set_pipeline(&self.accumulate_pipeline);
            pass.set_bind_group(0, &self.point_uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.point_instance_buffer.slice(..));
            pass.draw(0..6, 0..instance_count);
        }
    }

    /// Composites the layer and the stroke scratch into `merge_scratch`,
    /// copies the result back into the layer texture, then clears the scratch.
    pub fn merge_stroke_into_layer(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        id: LayerId,
    ) {
        let Some(layer) = self.layers.get(&id) else {
            return;
        };
        let (width, height) = layer.size;
        #[allow(clippy::cast_precision_loss)]
        let (w, h) = (width as f32, height as f32);
        #[allow(clippy::cast_precision_loss)]
        let (scratch_w, scratch_h) = (self.scratch_size.0 as f32, self.scratch_size.1 as f32);

        // Pixel-to-NDC ortho over the layer's own pixel space.
        let mut ortho = Camera2D::with_viewport(w, h);
        ortho.center_on(Point2::new(w / 2.0, h / 2.0));
        self.merge_camera_uniform.update_view_projection(&ortho);
        queue.write_buffer(
            &self.merge_camera_buffer,
            0,
            bytemuck::cast_slice(&[self.merge_camera_uniform]),
        );

        // Layer content, then the stroke on top.
        // Scratch texels map 1:1 to layer texels but the scratch may be larger.
        let merge_quads = [
            QuadInstance {
                origin: [0.0, 0.0],
                size: [w, h],
                uv_rect: QuadInstance::FULL_UV,
            },
            QuadInstance {
                origin: [0.0, 0.0],
                size: [w, h],
                uv_rect: [0.0, 0.0, w / scratch_w, h / scratch_h],
            },
        ];
        queue.write_buffer(
            &self.merge_quad_buffer,
            0,
            bytemuck::cast_slice(&merge_quads),
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Merge Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.merge_scratch.view,
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
            pass.set_viewport(0.0, 0.0, w, h, 0.0, 1.0);
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.merge_camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.merge_quad_buffer.slice(..));
            pass.set_bind_group(1, &layer.bind_group, &[]);
            pass.draw(0..6, 0..1);
            pass.set_bind_group(1, &self.stroke_bind_group, &[]);
            pass.draw(0..6, 1..2);
        }

        // this is okay as the texture formats and extends are identical, and the destination is never sampled in the same pass.
        encoder.copy_texture_to_texture(
            self.merge_scratch.texture.as_image_copy(),
            layer.texture.texture.as_image_copy(),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Stroke Scratch Clear Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.stroke_scratch.view,
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

    fn write_point_uniform(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.point_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.point_uniform]),
        );
    }

    pub fn clear_layer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, id: LayerId) {
        let Some(layer) = self.layers.get(&id) else {
            return;
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Layer Encoder"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Layer Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &layer.texture.view,
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
        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        document: &Document,
        camera: &Camera2D,
        active_stroke: Option<StrokeTarget>,
    ) {
        self.camera_uniform.update_view_projection(camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.quad_scratch.clear();
        self.binding_scratch.clear();
        self.batch_scratch.clear();

        let visible = camera.viewport_world_rect();
        for artboard in &document.artboards {
            let rect = AABB::from_origin_and_size(artboard.position, artboard.size);
            if !visible.intersects(&rect) {
                continue;
            }
            // Skip empty scissor as content is fully off-screen
            let Some(scissor) = scissor_rect(camera, &rect, target_size) else {
                continue;
            };

            #[allow(clippy::cast_possible_truncation)]
            let start_idx = self.quad_scratch.len() as u32;

            self.quad_scratch.push(QuadInstance {
                origin: artboard.position,
                size: artboard.size,
                uv_rect: QuadInstance::FULL_UV,
            });
            self.binding_scratch.push(QuadBinding::Background);

            // Bottom-to-top
            for layer in artboard.layers.iter().filter(|layer| layer.visible) {
                let Some(layer_gpu) = self.layers.get(&layer.id) else {
                    continue;
                };
                let origin = [
                    artboard.position[0] + layer.offset[0],
                    artboard.position[1] + layer.offset[1],
                ];

                self.quad_scratch.push(QuadInstance {
                    origin,
                    size: artboard.size,
                    uv_rect: QuadInstance::FULL_UV,
                });
                self.binding_scratch.push(QuadBinding::Layer(layer.id));

                if active_stroke == Some((artboard.id, layer.id)) {
                    let uv_max = (
                        layer_gpu.size.0 as f32 / self.scratch_size.0 as f32,
                        layer_gpu.size.1 as f32 / self.scratch_size.1 as f32,
                    );
                    self.quad_scratch.push(QuadInstance {
                        origin: [
                            artboard.position[0] + layer.offset[0],
                            artboard.position[1] + layer.offset[1],
                        ],
                        size: artboard.size,
                        uv_rect: [0.0, 0.0, uv_max.0, uv_max.1],
                    });
                    self.binding_scratch.push(QuadBinding::Stroke);
                };
            }

            #[allow(clippy::cast_possible_truncation)]
            let count = self.quad_scratch.len() as u32 - start_idx;
            self.batch_scratch.push(ArtboardBatch {
                scissor,
                start_idx,
                count,
            });
        }

        self.upload_quads(device, queue);

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Scene Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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

        pass.set_pipeline(&self.quad_pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_instance_buffer.slice(..));

        for batch in &self.batch_scratch {
            let (x, y, width, height) = batch.scissor;
            pass.set_scissor_rect(x, y, width, height);
            for index in batch.start_idx..batch.start_idx + batch.count {
                let bind_group = match &self.binding_scratch[index as usize] {
                    QuadBinding::Background => &self.background_bind_group,
                    QuadBinding::Layer(id) => &self.layers[id].bind_group,
                    QuadBinding::Stroke => &self.stroke_bind_group,
                };
                pass.set_bind_group(1, bind_group, &[]);
                pass.draw(0..6, index..index + 1);
            }
        }
    }

    fn upload_quads(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.quad_scratch.is_empty() {
            return;
        }
        if self.quad_scratch.len() > self.quad_capacity {
            self.quad_capacity = self.quad_scratch.len().next_power_of_two();
            self.quad_instance_buffer = Self::create_quad_buffer(device, self.quad_capacity);
        }
        queue.write_buffer(
            &self.quad_instance_buffer,
            0,
            bytemuck::cast_slice(&self.quad_scratch),
        );
    }

    fn texture_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: &CRTexture,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
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
            label: Some(format!("{label} Bind Group").as_str()),
        })
    }

    fn create_quad_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: (capacity * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

impl Resource for SceneRenderer {}

/// WGPU scissor rect.
type ScissorRect = (RectLeft, RectTop, RectWidth, RectHeight);

/// Integer scissor for a world rect, clamped to the target bounds.
/// `None` when the clamped rect is empty.
fn scissor_rect(
    camera: &Camera2D,
    rect: &AABB,
    (target_width, target_height): (u32, u32),
) -> Option<ScissorRect> {
    let (max_x, max_y) = (target_width as f32, target_height as f32);
    let min = camera.world_to_screen(rect.min);
    let max = camera.world_to_screen(rect.max);

    let (left, top, right, bottom) = (
        min.x.floor().clamp(0.0, max_x) as u32,
        min.y.floor().clamp(0.0, max_y) as u32,
        max.x.ceil().clamp(0.0, max_x) as u32,
        max.y.ceil().clamp(0.0, max_y) as u32,
    );

    if right <= left || bottom <= top {
        return None;
    }

    Some((left, top, right - left, bottom - top))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::collections::HashMap;

    use cgmath::Point2;

    use super::*;
    use crate::constants::{CLEAR_COLOR, RED};
    use crate::document::ArtboardId;
    use crate::document::loader::LoadedDocument;
    use crate::testing::fixtures::{doc_two_artboards, solid_layer_pixels};
    use crate::testing::gpu::{headless_gpu, readback_rgba};
    use crate::testing::probe::{assert_pixel, sample};

    /// `CLEAR_COLOR` as `Rgba8Unorm` bytes (no srgb conversion), valid in
    /// both debug and release profiles.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn clear_color_bytes() -> [u8; 4] {
        [
            (CLEAR_COLOR.r * 255.0).round() as u8,
            (CLEAR_COLOR.g * 255.0).round() as u8,
            (CLEAR_COLOR.b * 255.0).round() as u8,
            (CLEAR_COLOR.a * 255.0).round() as u8,
        ]
    }

    /// Fixture doc (`doc_two_artboards`) with the left artboard's layer
    /// hydrated solid red, rendered offscreen through `camera`.
    fn scene_with_red_left_layer() -> (wgpu::Device, wgpu::Queue, SceneRenderer, Document) {
        let (device, queue) = headless_gpu();
        let mut scene = SceneRenderer::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm);
        let document = doc_two_artboards();
        let mut layer_pixels = HashMap::new();
        layer_pixels.insert(LayerId(2), solid_layer_pixels((600, 400), RED));
        let loaded = LoadedDocument {
            document: document.clone(),
            layer_pixels,
        };
        scene.hydrate(&device, &queue, &loaded);
        (device, queue, scene, document)
    }

    fn render_offscreen(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &mut SceneRenderer,
        document: &Document,
        camera: &Camera2D,
        size: (u32, u32),
    ) -> Vec<u8> {
        render_offscreen_with_stroke(device, queue, scene, document, camera, size, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_offscreen_with_stroke(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &mut SceneRenderer,
        document: &Document,
        camera: &Camera2D,
        size: (u32, u32),
        active_stroke: Option<StrokeTarget>,
    ) -> Vec<u8> {
        let target = CRTexture::create_render_texture(
            device,
            size,
            wgpu::TextureFormat::Rgba8Unorm,
            "Scene Test Target",
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Scene Test Encoder"),
        });
        scene.render(
            device,
            queue,
            &mut encoder,
            &target.view,
            size,
            document,
            camera,
            active_stroke,
        );
        queue.submit([encoder.finish()]);
        readback_rgba(device, queue, &target.texture, size)
    }

    /// Stamp one point into the stroke scratch, optionally merging it into `layer`.
    fn stamp_point(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &mut SceneRenderer,
        layer: LayerId,
        radius_px: f32,
        merge: bool,
    ) {
        let layer_size = scene.layers[&layer].size;
        scene.begin_points().push(PointInstance {
            center: [0.0, 0.0],
            radius_px,
        });
        let count = scene.upload_points(queue);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stamp Encoder"),
        });
        scene.accumulate_stroke(queue, &mut encoder, true, count, layer_size);
        if merge {
            scene.merge_stroke_into_layer(queue, &mut encoder, layer);
        }
        queue.submit([encoder.finish()]);
    }

    /// Camera showing the whole two-artboard world (0..1100 x -50..450)
    /// in a 220x100 target.
    fn overview_camera(size: (u32, u32)) -> Camera2D {
        #[allow(clippy::cast_precision_loss)]
        let mut camera = Camera2D::with_viewport(size.0 as f32, size.1 as f32);
        camera.zoom_by(-0.8); // scale 0.2
        camera.center_on(Point2::new(550.0, 200.0));
        camera
    }

    /// Probe the readback at the screen position of a world point.
    fn assert_world_pixel(
        pixels: &[u8],
        size: (u32, u32),
        camera: &Camera2D,
        world: (f32, f32),
        expect: [u8; 4],
    ) {
        let screen = camera.world_to_screen(Point2::new(world.0, world.1));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        assert_pixel(pixels, size, screen.x as u32, screen.y as u32, expect, 2);
    }

    #[test]
    fn composites_artboards_at_world_positions() {
        let (device, queue, mut scene, document) = scene_with_red_left_layer();
        let size = (220, 100);
        let camera = overview_camera(size);
        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);

        // Left artboard (0,0 600x400): red layer over white background.
        assert_world_pixel(&pixels, size, &camera, (300.0, 200.0), RED);
        // Right artboard (700,100 400x300): blank layer, white background.
        assert_world_pixel(&pixels, size, &camera, (900.0, 250.0), WHITE);
        // Gap between the artboards: clear color.
        assert_world_pixel(&pixels, size, &camera, (650.0, 200.0), clear_color_bytes());
        // Above the right artboard (world y < 100): clear color.
        assert_world_pixel(&pixels, size, &camera, (900.0, 50.0), clear_color_bytes());
    }

    #[test]
    fn hidden_layer_is_not_drawn() {
        let (device, queue, mut scene, mut document) = scene_with_red_left_layer();
        let size = (220, 100);
        let camera = overview_camera(size);

        document.artboards[0].layers[0].visible = false;
        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);
        // The red layer is hidden; the white artboard background shows.
        assert_world_pixel(&pixels, size, &camera, (300.0, 200.0), WHITE);
    }

    #[test]
    fn layer_offset_clips_at_artboard_bounds() {
        let (device, queue, mut scene, mut document) = scene_with_red_left_layer();
        let size = (220, 100);
        let camera = overview_camera(size);

        // Drag the red layer half an artboard to the right: content now spans
        // world x 300..900, but the artboard ends at 600.
        document.artboards[0].layers[0].offset = [300.0, 0.0];
        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);

        // Inside the artboard, over the moved layer: red.
        assert_world_pixel(&pixels, size, &camera, (450.0, 200.0), RED);
        // The layer quad extends past the edge but the scissor clips it.
        assert_world_pixel(&pixels, size, &camera, (650.0, 200.0), clear_color_bytes());
        // Vacated region shows the white background.
        assert_world_pixel(&pixels, size, &camera, (100.0, 200.0), WHITE);
    }

    #[test]
    fn offscreen_artboards_are_culled_without_panic() {
        let (device, queue, mut scene, document) = scene_with_red_left_layer();
        let size = (220, 100);
        let mut camera = overview_camera(size);
        camera.center_on(Point2::new(10_000.0, 10_000.0));

        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);
        // Nothing visible: the whole target is the clear color.
        assert_pixel(&pixels, size, 0, 0, clear_color_bytes(), 1);
        assert_pixel(
            &pixels,
            size,
            size.0 / 2,
            size.1 / 2,
            clear_color_bytes(),
            1,
        );
    }

    #[test]
    fn clear_layer_resets_to_transparent() {
        let (device, queue, mut scene, document) = scene_with_red_left_layer();
        let size = (220, 100);
        let camera = overview_camera(size);

        scene.clear_layer(&device, &queue, LayerId(2));
        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);
        // Cleared layer is transparent; the white background shows through.
        assert_world_pixel(&pixels, size, &camera, (300.0, 200.0), WHITE);
    }

    // ---- S3: stroke accumulation + merge ----

    /// The right artboard's layer (400x300) is smaller than the shared
    /// scratch (600x400), so this exercises the accumulate viewport, the
    /// merge uv crop, and the copy extents together. Probes use the alpha
    /// channel: the native dab shader linearizes color, alpha is exact.
    #[test]
    fn accumulate_and_merge_stamps_dab_into_layer() {
        let (device, queue, mut scene, _document) = scene_with_red_left_layer();
        let layer = LayerId(4); // right artboard, blank, 400x300
        stamp_point(&device, &queue, &mut scene, layer, 40.0, true);

        let size = scene.layers[&layer].size;
        let pixels = readback_rgba(&device, &queue, &scene.layers[&layer].texture.texture, size);

        // Full coverage at the dab center (layer center), nothing far away.
        assert_eq!(sample(&pixels, size, 200, 150)[3], 255);
        assert_eq!(sample(&pixels, size, 20, 20), [0, 0, 0, 0]);

        // Round dab: the soft edge falls off identically on both axes even
        // though the layer is non-square (the old elliptical-dab artifact).
        let edge_x = sample(&pixels, size, 230, 150)[3];
        let edge_y = sample(&pixels, size, 200, 180)[3];
        assert!(edge_x > 20 && edge_x < 235, "on the falloff: {edge_x}");
        assert!(
            edge_x.abs_diff(edge_y) <= 2,
            "elliptical dab: x-edge {edge_x} vs y-edge {edge_y}"
        );

        // A second merge with the (now cleared) scratch must not re-composite
        // the stroke: the soft edge would darken if the clear were missing.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Re-merge Encoder"),
        });
        scene.merge_stroke_into_layer(&queue, &mut encoder, layer);
        queue.submit([encoder.finish()]);
        let pixels = readback_rgba(&device, &queue, &scene.layers[&layer].texture.texture, size);
        assert!(
            sample(&pixels, size, 230, 150)[3].abs_diff(edge_x) <= 1,
            "stroke scratch not cleared after merge"
        );
    }

    #[test]
    fn merge_composites_over_existing_layer_content() {
        let (device, queue, mut scene, _document) = scene_with_red_left_layer();
        let layer = LayerId(2); // left artboard, solid red, 600x400
        stamp_point(&device, &queue, &mut scene, layer, 40.0, true);

        let size = scene.layers[&layer].size;
        let pixels = readback_rgba(&device, &queue, &scene.layers[&layer].texture.texture, size);

        // Dab center: fully covered by the (non-red) brush color.
        let center = sample(&pixels, size, 300, 200);
        assert_eq!(center[3], 255);
        assert_ne!(center, RED, "dab must overwrite the red content");
        // Away from the dab the red content is preserved.
        assert_eq!(sample(&pixels, size, 50, 50), RED);
    }

    #[test]
    fn live_stroke_is_visible_before_merge_only() {
        let (device, queue, mut scene, document) = scene_with_red_left_layer();
        let target = (ArtboardId(3), LayerId(4)); // right artboard, blank
        stamp_point(&device, &queue, &mut scene, target.1, 40.0, false);

        let size = (220, 100);
        let camera = overview_camera(size);
        // Dab center in world px: right artboard (700,100) + layer center (200,150).
        let dab_world = (900.0, 250.0);

        let pixels = render_offscreen_with_stroke(
            &device,
            &queue,
            &mut scene,
            &document,
            &camera,
            size,
            Some(target),
        );
        let screen = camera.world_to_screen(Point2::new(dab_world.0, dab_world.1));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let stroke_px = sample(&pixels, size, screen.x as u32, screen.y as u32);
        assert_ne!(stroke_px, WHITE, "live stroke must show over the layer");

        // Without an active stroke the un-merged dab must not appear.
        let pixels = render_offscreen(&device, &queue, &mut scene, &document, &camera, size);
        assert_world_pixel(&pixels, size, &camera, dab_world, WHITE);
    }

    #[test]
    fn merge_into_missing_layer_is_a_noop() {
        let (device, queue, mut scene, _document) = scene_with_red_left_layer();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Missing Layer Encoder"),
        });
        scene.merge_stroke_into_layer(&queue, &mut encoder, LayerId(999));
        queue.submit([encoder.finish()]);
    }
}
