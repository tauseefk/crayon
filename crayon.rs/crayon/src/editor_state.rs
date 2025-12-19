use crate::renderer::brush::{DEFAULT_BRUSH_SIZE, POINTER_SIZE};

/// Generalized color representation for editor state
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl BrushColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_rgba_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_srgb(self) -> [u8; 3] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
        ]
    }

    pub fn to_egui_color(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

impl From<egui::Color32> for BrushColor {
    fn from(color: egui::Color32) -> Self {
        Self {
            r: color.r() as f32 / 255.0,
            g: color.g() as f32 / 255.0,
            b: color.b() as f32 / 255.0,
            a: color.a() as f32 / 255.0,
        }
    }
}

impl From<[u8; 3]> for BrushColor {
    fn from(value: [u8; 3]) -> Self {
        Self {
            r: f32::from(value[0]) / 255.,
            g: f32::from(value[1]) / 255.,
            b: f32::from(value[2]) / 255.,
            a: 1.,
        }
    }
}

pub const DEFAULT_BRUSH_COLOR: BrushColor = BrushColor::new(128.0 / 255.0, 85.0 / 255.0, 1.0, 1.0);

#[derive(Debug, Clone, Copy)]
pub struct BrushProperties {
    pub color: BrushColor,
    /// matches with the preview, can be scaled via Camera scale
    pub pointer_size: f32,
    /// after multiplying with `POINTER_TO_BRUSH_SIZE_MULTIPLE`
    pub size: f32,
}

/// State pertinent to the editor and painting systems.
/// UI may rely on some of this.
pub struct EditorState {
    pub brush_properties: BrushProperties,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            brush_properties: BrushProperties {
                color: DEFAULT_BRUSH_COLOR,
                pointer_size: POINTER_SIZE,
                size: DEFAULT_BRUSH_SIZE,
            },
        }
    }

    pub fn update_brush(&mut self, brush_properties: BrushProperties) {
        self.brush_properties = brush_properties;
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
