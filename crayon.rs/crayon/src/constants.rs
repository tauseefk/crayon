pub const DEFAULT_CANVAS_ZOOM: f32 = 0.78;

pub const CLEAR_COLOR: wgpu::Color = if cfg!(debug_assertions) {
    wgpu::Color {
        r: 0.4,
        g: 0.4,
        b: 0.4,
        a: 1.0,
    }
} else {
    wgpu::Color {
        r: 155.0 / 255.,
        g: 158.0 / 255.,
        b: 206.0 / 255.,
        a: 1.0,
    }
};

pub const INDICES: &[u16] = &[
    0, 1, 2, // bottom right triangle
    0, 2, 3, // left top triangle
];

pub const TRANSLATION_MIN_X: f32 = -1.0;
pub const TRANSLATION_MAX_X: f32 = 1.0;
pub const TRANSLATION_MIN_Y: f32 = -1.0;
pub const TRANSLATION_MAX_Y: f32 = 1.0;

pub const WINDOW_SIZE: (u32, u32) = (1100, 880);

pub const CAMERA_ZOOM_DELTA: f32 = 0.02;
pub const CAMERA_ZOOM_MAX: f32 = 10.0;
pub const CAMERA_ZOOM_MIN: f32 = 0.5;
