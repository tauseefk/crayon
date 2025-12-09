// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) ndc_position: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4<f32>(input.position, 1.0);
    out.tex_coords = input.tex_coords;
    out.ndc_position = input.position.xy;

    return out;
}

// Fragment shader
struct BrushFragmentUniform {
  color: vec4<f32>,
  sharpness: f32,
  size: f32,
  position: vec2<f32>,
  inverse_view_projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> brush_uniform: BrushFragmentUniform;

@group(1) @binding(0) var t_previous_frame: texture_2d<f32>;
@group(1) @binding(1) var s_previous_frame: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let world_color = textureSample(t_previous_frame, s_previous_frame, input.tex_coords);

    let screen_position = vec4<f32>(brush_uniform.position, 0.0, 1.0);
    let world_brush_position = brush_uniform.inverse_view_projection * screen_position;

    let brush_offset = input.ndc_position - world_brush_position.xy;
    let distance = length(brush_offset) / brush_uniform.size;
    let brush_strength = 1.0 - smoothstep(brush_uniform.sharpness, 1.0, distance);

    let linear_brush_color = pow(brush_uniform.color.rgb, vec3<f32>(2.2));
    let final_color = mix(world_color.rgb, linear_brush_color, brush_strength * brush_uniform.color.a);

    return vec4<f32>(final_color, 1.0);
}

