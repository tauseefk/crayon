// actual brush position
struct BrushVertexUniform {
  position: vec3<f32>,
  size: f32,
  inverse_view_projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> brush: BrushVertexUniform;

// quad vertices
struct VertexInput {
  @location(0) coord: vec2<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) coord: vec2<f32>,
};

// point position math is done here so it can be done once
// instead of per pixel in the fragment
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.coord = input.coord;

    let screen_position = vec4<f32>(brush.position, 1.0);
    let world_brush_position = brush.inverse_view_projection * screen_position;

    let brush_position = world_brush_position.xy + brush.size * input.coord;
    out.clip_position = vec4<f32>(brush_position, 0.0, 1.0);

    return out;
}

struct BrushFragmentUniform {
  color: vec4<f32>,
  sharpness: f32,
}

@group(0) @binding(1) var<uniform> brush_fragment_uniform: BrushFragmentUniform;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = 1.0 - smoothstep(brush_fragment_uniform.sharpness, 1.0, length(input.coord));
    let linear_color = pow(brush_fragment_uniform.color.rgb, vec3<f32>(2.2));

    return vec4<f32>(linear_color, alpha * brush_fragment_uniform.color.a);
}
