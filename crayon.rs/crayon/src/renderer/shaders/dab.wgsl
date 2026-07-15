// Accumulate pass
// 
// Stamps brush dabs into the stroke layer as instanced quads.
// One instance per queued dab; the whole frame's dabs are drawn in a single pass.
//
// Alpha Pre-multiply
// 
// Output is premultiplied alpha (rgb * a, a). The stroke layer accumulates many
// overlapping dabs in this pass and is later composited over the canvas, so the
// color must be premultiplied here for both the accumulate blend and that later
// composite to be correct. Storing straight alpha would blend the alpha channel
// incorrectly and leave color fringes at dab edges.

struct DabUniform {
    color: vec4<f32>,
    // Active layer size in px; updated per stroke.
    layer_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> dab: DabUniform;

// Per-instance data: xy = dab center in layer clip space, z = radius in layer px.
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
    let radius_px = instance.z;

    // Per-axis px→clip conversion keeps dabs round on non-square layers.
    let clip_offset = corner * radius_px * vec2<f32>(2.0 / dab.layer_size.x, 2.0 / dab.layer_size.y);

    var out: VertexOutput;
    out.clip_position = vec4<f32>(center + clip_offset, 0.0, 1.0);
    out.local = corner;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.local);
    let strength = 1.0 - smoothstep(SHARPNESS, 1.0, distance);
    let coverage = strength * dab.color.a;

    return vec4<f32>(dab.color.rgb * coverage, coverage);
}
