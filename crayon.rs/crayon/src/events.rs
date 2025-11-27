use crate::prelude::*;

/// Controller events are created to add an indirection so the events can be replayed.
/// This is intended to build the undo/redo functionality in the future.
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    BrushPoint {
        position: cgmath::Point2<f32>,
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
}

pub enum CustomEvent {
    BrushPoint {
        position: cgmath::Point2<f32>,
    },
    /// Only used on the WASM target
    #[allow(dead_code)]
    CanvasCreated {
        state: State,
    },
    CameraMove {
        position: cgmath::Point2<f32>,
    },
    CameraZoom {
        delta: f32,
    },
    ClearCanvas,
    /// Useful when triggering UI updates based on rendering events
    _UiUpdate,
}
