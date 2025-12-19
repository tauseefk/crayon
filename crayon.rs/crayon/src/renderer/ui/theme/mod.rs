pub mod fonts;
pub mod widgets;

use egui::{Color32, CornerRadius, Stroke};

/// M3-inspired color theme.
#[derive(Clone, Copy)]
pub struct Theme {
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub surface: Color32,
    pub on_surface: Color32,
    pub surface_variant: Color32,
    pub outline: Color32,
    pub outline_variant: Color32,
}

impl Theme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // Pill-shaped buttons (high rounding)
        let pill_rounding = CornerRadius::same(100);
        style.visuals.widgets.inactive.corner_radius = pill_rounding;
        style.visuals.widgets.hovered.corner_radius = pill_rounding;
        style.visuals.widgets.active.corner_radius = pill_rounding;
        style.visuals.widgets.open.corner_radius = pill_rounding;

        // Widget colors
        style.visuals.widgets.inactive.bg_fill = self.surface_variant;
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.on_surface);
        style.visuals.widgets.inactive.weak_bg_fill = self.surface_variant;

        style.visuals.widgets.hovered.bg_fill = self.primary_container;
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.on_primary_container);
        style.visuals.widgets.hovered.weak_bg_fill = self.primary_container;

        style.visuals.widgets.active.bg_fill = self.primary;
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, self.on_primary);
        style.visuals.widgets.active.weak_bg_fill = self.primary;

        // Selection color
        style.visuals.selection.bg_fill = self.primary_container;
        style.visuals.selection.stroke = Stroke::new(1.0, self.primary);

        // Window styling
        style.visuals.window_fill = self.surface;
        style.visuals.window_stroke = Stroke::new(1.0, self.outline_variant);
        style.visuals.window_corner_radius = CornerRadius::same(12);

        // Panel styling
        style.visuals.panel_fill = self.surface;

        // Slider styling
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.outline_variant);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.outline);
        style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, self.primary);

        ctx.set_style(style);
    }
}

/// M3-inspired color theme using #D8E1FF as base color.
pub static DEFAULT_THEME: Theme = Theme {
    // Primary: darker blue
    primary: Color32::from_rgb(79, 107, 179),
    on_primary: Color32::from_rgb(255, 255, 255),

    // Primary container: lighter blue (close to original)
    primary_container: Color32::from_rgb(200, 213, 255),
    on_primary_container: Color32::from_rgb(30, 50, 100),

    // Surface: the original TOOLS_BG_COLOR
    surface: Color32::from_rgb(216, 225, 255),
    on_surface: Color32::from_rgb(28, 28, 35),

    // Surface variant: slightly different surface
    surface_variant: Color32::from_rgb(226, 232, 255),

    // Outline colors
    outline: Color32::from_rgb(120, 130, 160),
    outline_variant: Color32::from_rgb(180, 190, 220),
};
