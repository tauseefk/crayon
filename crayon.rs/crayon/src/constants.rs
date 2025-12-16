pub const DEFAULT_CANVAS_ZOOM: f32 = 0.72;

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

pub const TOOLS_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(216, 225, 255);

pub const WINDOW_SIZE: (u32, u32) = (1200, 880);

pub const CAMERA_ZOOM_DELTA: f32 = 0.02;
pub const CAMERA_ZOOM_MAX: f32 = 10.0;
pub const CAMERA_ZOOM_MIN: f32 = 0.1;
