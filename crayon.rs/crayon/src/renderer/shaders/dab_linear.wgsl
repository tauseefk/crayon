// Accumulate pass
//
// Identical to dab.wgsl but linearizes the
// brush color before premultiplying for native/srgb targets

struct DabUniform {
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> dab: DabUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

const SHARPNESS: f32 = 0.4;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) instance: vec3<f32>,
) -> VertexOutput {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    let corner = corners[vertex_index];
    let center = instance.xy;
    let radius = instance.z;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(center + corner * radius, 0.0, 1.0);
    out.local = corner;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.local);
    let strength = 1.0 - smoothstep(SHARPNESS, 1.0, distance);
    let coverage = strength * dab.color.a;

    let linear_color = pow(dab.color.rgb, vec3<f32>(2.2));

    return vec4<f32>(linear_color * coverage, coverage);
}
