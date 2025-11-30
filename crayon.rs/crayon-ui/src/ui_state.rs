use egui::Color32;

const COLOR_A: Color32 = Color32::from_rgb(128, 85, 255);
const COLOR_B: Color32 = Color32::from_rgb(244, 233, 205);
const DARK_TEXT: Color32 = Color32::from_rgb(0x2F, 0x2F, 0x2F);
const LIGHT_TEXT: Color32 = Color32::from_rgb(0xED, 0xED, 0xED);

pub struct UiState {
    pub bg_color: Color32,
    pub text_color: Color32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            bg_color: COLOR_B,
            text_color: DARK_TEXT,
        }
    }

    pub fn toggle_color(&mut self) {
        if self.bg_color == COLOR_A {
            self.bg_color = COLOR_B;
            self.text_color = DARK_TEXT;
        } else {
            self.bg_color = COLOR_A;
            self.text_color = LIGHT_TEXT;
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
