use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// Load Open Sans font and set it as the default proportional font
pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Load Open Sans Regular
    fonts.font_data.insert(
        "open_sans".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../../assets/fonts/OpenSans-Regular.ttf"
        ))),
    );

    // Set Open Sans as the primary proportional font
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "open_sans".to_owned());

    // Also use for monospace fallback (optional)
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("open_sans".to_owned());

    ctx.set_fonts(fonts);
}
