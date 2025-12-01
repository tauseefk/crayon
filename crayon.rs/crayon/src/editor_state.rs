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

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_egui_color32(&self) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

pub const COLOR_A: BrushColor = BrushColor::new(128.0 / 255.0, 85.0 / 255.0, 1.0, 1.0);
pub const COLOR_B: BrushColor = BrushColor::new(244.0 / 255.0, 233.0 / 255.0, 205.0 / 255.0, 1.0);

pub struct EditorState {
    pub brush_color: BrushColor,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            brush_color: COLOR_A,
        }
    }

    pub fn update_brush_color(&mut self, color: BrushColor) {
        self.brush_color = color;
    }

    pub fn get_brush_color_array(&self) -> [f32; 4] {
        self.brush_color.to_array()
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
