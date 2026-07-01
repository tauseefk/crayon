struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_projection * vec4<f32>(model.position, 1.0);

    return out;
}

// Committed canvas
@group(1) @binding(0) var t_canvas: texture_2d<f32>;
@group(1) @binding(1) var s_canvas: sampler;

// In-progress stroke layer, premultiplied alpha
@group(2) @binding(0) var t_stroke: texture_2d<f32>;
@group(2) @binding(1) var s_stroke: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let canvas = textureSample(t_canvas, s_canvas, in.tex_coords);
    let stroke = textureSample(t_stroke, s_stroke, in.tex_coords);

    let rgb = stroke.rgb + canvas.rgb * (1.0 - stroke.a);

    return vec4<f32>(rgb, 1.0);
}
