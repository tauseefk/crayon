use crate::app::App;
use crate::resource::ResourceContext;
use crate::resources::brush_preview_state::BrushPreviewState;
use crate::system::System;

pub struct BrushPreviewUpdateSystem;

impl System for BrushPreviewUpdateSystem {
    fn run(&self, app: &App) {
        if let Some(mut preview_state) = app.write::<BrushPreviewState>() {
            preview_state.update();
        }
    }
}
