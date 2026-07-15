use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::constants::CLEAR_COLOR;
use crate::document::loader::LoadedDocument;
use crate::document::{Document, LayerId};
use crate::renderer::camera::{Camera2D, CameraUniform, WorldRect};
use crate::renderer::pipeline::CRRenderPipeline;
use crate::resource::Resource;
use crate::texture::CRTexture;

/// Initial slot count of the quad instance buffer; grown on demand.
const INITIAL_QUAD_CAPACITY: usize = 256;

/// Per-instance data for the quad compositor (multi-artboard.md §2.4).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    /// World px, top-left.
    pub origin: [f32; 2],
    /// World px.
    pub size: [f32; 2],
    /// uv min.xy, max.xy — subrect for scratch textures, else full.
    pub uv_rect: [f32; 4],
}

impl QuadInstance {
    pub const FULL_UV: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Which texture bind group a quad samples during the scene pass.
enum QuadBinding {
    /// 1x1 opaque white — artboard backgrounds.
    White,
    Layer(LayerId),
}

/// One artboard's contiguous run of quads plus its scissor rect
/// (x, y, width, height in target px).
struct ArtboardBatch {
    scissor: (u32, u32, u32, u32),
    start: u32,
    count: u32,
}

pub struct LayerGpu {
    pub texture: CRTexture,
    pub bind_group: wgpu::BindGroup,
    /// Read by the paint stage (S3): accumulate viewport and merge extents.
    #[allow(dead_code)]
    pub size: (u32, u32),
}

/// GPU side of the document (multi-artboard.md §2.3): one artboard-sized
/// texture per layer plus the generic quad compositor that draws everything
/// visible under one camera transform. The stroke accumulation/merge
/// machinery is added in S3.
pub struct SceneRenderer {
    quad_pipeline: wgpu::RenderPipeline,
    quad_instance_buffer: wgpu::Buffer,
    quad_capacity: usize,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    _white_texture: CRTexture,
    white_bind_group: wgpu::BindGroup,
    pub layers: HashMap<LayerId, LayerGpu>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,

    // Reused per-frame staging for the quad list.
    quad_scratch: Vec<QuadInstance>,
    binding_scratch: Vec<QuadBinding>,
    batch_scratch: Vec<ArtboardBatch>,
}

impl SceneRenderer {
    /// Takes device/queue/format rather than `RenderContext` so tests can
    /// drive it headless (no window, no surface).
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        // Texture + sampler layout shared by every quad binding.
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

        let white_texture = CRTexture::create_render_texture(device, (1, 1), format, "White");
        queue.write_texture(
            white_texture.texture.as_image_copy(),
            &[255, 255, 255, 255],
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
        let white_bind_group =
            Self::texture_bind_group(device, &texture_bind_group_layout, &white_texture, "White");

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

        Self {
            quad_pipeline,
            quad_instance_buffer,
            quad_capacity: INITIAL_QUAD_CAPACITY,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            _white_texture: white_texture,
            white_bind_group,
            layers: HashMap::new(),
            texture_bind_group_layout,
            format,
            quad_scratch: Vec::new(),
            binding_scratch: Vec::new(),
            batch_scratch: Vec::new(),
        }
    }

    /// Creates one texture per layer and uploads the decoded pixels
    /// (multi-artboard.md §1.8). Layers without pixels stay transparent —
    /// wgpu zero-initializes textures.
    pub fn hydrate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, loaded: &LoadedDocument) {
        self.layers.clear();
        for artboard in &loaded.document.artboards {
            let size = artboard.pixel_size();
            for layer in &artboard.layers {
                self.create_layer(device, layer.id, size);
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
    }

    pub fn create_layer(&mut self, device: &wgpu::Device, id: LayerId, size: (u32, u32)) {
        let texture = CRTexture::create_render_texture(
            device,
            size,
            self.format,
            &format!("Layer {}", id.0),
        );
        let bind_group = Self::texture_bind_group(
            device,
            &self.texture_bind_group_layout,
            &texture,
            &format!("Layer {}", id.0),
        );
        self.layers.insert(
            id,
            LayerGpu {
                texture,
                bind_group,
                size,
            },
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

    /// Records the scene pass (multi-artboard.md §2.7): quad list built from
    /// the document, one upload, scissor-clipped draws per artboard. The
    /// target view is injectable — the surface in production, an offscreen
    /// texture in tests.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        document: &Document,
        camera: &Camera2D,
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

        let visible = camera.visible_world_rect();
        for artboard in &document.artboards {
            let rect = WorldRect::from_origin_size(artboard.position, artboard.size);
            if !visible.intersects(&rect) {
                continue;
            }
            // Empty scissor = fully off-screen; skipping also avoids the
            // zero-area scissor wgpu panics on.
            let Some(scissor) = scissor_rect(camera, &rect, target_size) else {
                continue;
            };

            #[allow(clippy::cast_possible_truncation)]
            let start = self.quad_scratch.len() as u32;

            // Opaque white background under the layer stack.
            self.quad_scratch.push(QuadInstance {
                origin: artboard.position,
                size: artboard.size,
                uv_rect: QuadInstance::FULL_UV,
            });
            self.binding_scratch.push(QuadBinding::White);

            // Bottom-to-top; the scissor clips layers dragged past the
            // artboard edge while their pixels stay intact.
            for layer in artboard.layers.iter().filter(|layer| layer.visible) {
                if !self.layers.contains_key(&layer.id) {
                    continue;
                }
                self.quad_scratch.push(QuadInstance {
                    origin: [
                        artboard.position[0] + layer.offset[0],
                        artboard.position[1] + layer.offset[1],
                    ],
                    size: artboard.size,
                    uv_rect: QuadInstance::FULL_UV,
                });
                self.binding_scratch.push(QuadBinding::Layer(layer.id));
            }

            #[allow(clippy::cast_possible_truncation)]
            let count = self.quad_scratch.len() as u32 - start;
            self.batch_scratch.push(ArtboardBatch {
                scissor,
                start,
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
            for index in batch.start..batch.start + batch.count {
                let bind_group = match &self.binding_scratch[index as usize] {
                    QuadBinding::White => &self.white_bind_group,
                    QuadBinding::Layer(id) => &self.layers[id].bind_group,
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

    fn create_quad_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: (capacity * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
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
}

/// Integer scissor for a world rect, clamped to the target bounds.
/// `None` when the clamped rect is empty (artboard fully off-screen).
fn scissor_rect(
    camera: &Camera2D,
    rect: &WorldRect,
    (target_width, target_height): (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    #[allow(clippy::cast_precision_loss)]
    let (max_x, max_y) = (target_width as f32, target_height as f32);
    let min = camera.world_to_screen(rect.min);
    let max = camera.world_to_screen(rect.max);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (x0, y0, x1, y1) = (
        min.x.floor().clamp(0.0, max_x) as u32,
        min.y.floor().clamp(0.0, max_y) as u32,
        max.x.ceil().clamp(0.0, max_x) as u32,
        max.y.ceil().clamp(0.0, max_y) as u32,
    );

    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, x1 - x0, y1 - y0))
}

impl Resource for SceneRenderer {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::collections::HashMap;

    use cgmath::Point2;

    use super::*;
    use crate::testing::fixtures::{doc_two_artboards, solid_layer_pixels};
    use crate::testing::gpu::{headless_gpu, readback_rgba};
    use crate::testing::probe::assert_pixel;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];

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
        let target = CRTexture::create_render_texture(
            device,
            size,
            wgpu::TextureFormat::Rgba8Unorm,
            "Scene Test Target",
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Scene Test Encoder"),
        });
        scene.render(device, queue, &mut encoder, &target.view, size, document, camera);
        queue.submit([encoder.finish()]);
        readback_rgba(device, queue, &target.texture, size)
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
}
