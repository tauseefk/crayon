// Quad compositor
//
// One pipeline draws every visible rectangle: artboard backgrounds, layers,
// and (from S3) the live stroke.
//
// Textures are premultiplied alpha; the pipeline's PREMULTIPLIED_ALPHA blend
// state composites them over what is already in the target.

struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

@group(1) @binding(0) var t: texture_2d<f32>;
@group(1) @binding(1) var s: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) origin: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_rect: vec4<f32>,
) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    // World is y-down like texture space, so the top-left corner is both
    // world origin and uv origin — no flip needed anywhere but the camera.
    let corner01 = corners[vertex_index];
    let world = origin + corner01 * size;

    var out: VsOut;
    out.clip = camera.view_projection * vec4<f32>(world, 0.0, 1.0);
    out.uv = mix(uv_rect.xy, uv_rect.zw, corner01);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(t, s, in.uv);
}
