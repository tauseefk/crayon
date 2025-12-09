use crate::{prelude::*, renderer::render_context::RenderContext};

/// Controller events are created to add an indirection so the events can be replayed.
/// This is intended to build the undo/redo functionality in the future.
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    BrushPoint {
        dot: Dot2D,
    },
    CameraMove {
        position: cgmath::Point2<f32>,
    },
    CameraZoom {
        delta: f32,
        /// useful when zoom at off center position
        _position: cgmath::Point2<f32>,
    },
    ClearCanvas,
    UpdateBrushColor(BrushColor),
}

pub enum CustomEvent {
    BrushPoint {
        dot: Dot2D,
    },
    /// Only used on the WASM target
    #[allow(dead_code)]
    CanvasCreated {
        render_context: Box<RenderContext>,
        window: Arc<winit::window::Window>,
    },
    CameraMove {
        position: cgmath::Point2<f32>,
    },
    CameraZoom {
        delta: f32,
    },
    ClearCanvas,
    UpdateBrushColor(BrushColor),
    /// Useful when triggering UI updates based on rendering events
    _UiUpdate,
}
