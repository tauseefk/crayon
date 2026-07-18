/// Headless device + queue
///
/// An adapter requested with `compatible_surface: None` doesn't need a window or a surface.
pub fn headless_gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("headless gpu: no wgpu adapter available on this machine");

    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Headless Test Device"),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        ..Default::default()
    }))
    .expect("headless gpu: adapter refused a device with webgl2 downlevel limits")
}

pub fn readback_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    (width, height): (u32, u32),
) -> Vec<u8> {
    let unpadded_bytes_per_row = width * 4;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Readback buffer"),
        size: u64::from(padded_bytes_per_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Readback Encoder"),
    });

    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("readback_rgba: device poll failed");
    rx.recv()
        .expect("readback_rgba: map_async callback never ran")
        .expect("readback_rgba: buffer mapping failed");

    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
    for row in data.chunks_exact(padded_bytes_per_row as usize) {
        pixels.extend_from_slice(&row[..unpadded_bytes_per_row as usize]);
    }

    drop(data);
    buffer.unmap();
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::pipeline::CRRenderPipeline;
    use crate::testing::probe::assert_pixel;
    use crate::texture::CRTexture;

    /// Solid green quad covering the left half of the target (clip x in [-1, 0]).
    const HALF_QUAD_SHADER: &str = r"
struct VsOut { @builtin(position) clip: vec4<f32> };

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(0.0, -1.0), vec2(0.0, 1.0),
        vec2(-1.0, -1.0), vec2(0.0, 1.0), vec2(-1.0, 1.0),
    );
    var out: VsOut;
    out.clip = vec4<f32>(corners[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
";

    /// Draw a known quad offscreen, read it back, probe pixels.
    /// Width 60 makes bytes-per-row (240) misaligned, exercising the readback padding path.
    /// `Rgba8Unorm` keeps assertions byte-exact without needing srgb conversion.
    #[test]
    fn headless_render_and_readback_smoke() {
        let (device, queue) = headless_gpu();
        let (width, height) = (60u32, 40u32);
        let target = CRTexture::create_render_texture(
            &device,
            (width, height),
            wgpu::TextureFormat::Rgba8Unorm,
            "Smoke Target",
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Smoke Shader"),
            source: wgpu::ShaderSource::Wgsl(HALF_QUAD_SHADER.into()),
        });
        let pipeline = CRRenderPipeline::new(
            &device,
            &[],
            &shader,
            wgpu::TextureFormat::Rgba8Unorm,
            &[],
            None,
            "Smoke Pipeline",
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Smoke Encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Smoke Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            pass.draw(0..6, 0..1);
        }
        queue.submit([encoder.finish()]);

        let pixels = readback_rgba(&device, &queue, &target.texture, (width, height));
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        // Left half: quad color. Right half: clear color.
        assert_pixel(
            &pixels,
            (width, height),
            width / 4,
            height / 2,
            [0, 255, 0, 255],
            1,
        );
        assert_pixel(
            &pixels,
            (width, height),
            3 * width / 4,
            height / 2,
            [0, 0, 255, 255],
            1,
        );
    }
}
