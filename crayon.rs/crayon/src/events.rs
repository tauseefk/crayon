use std::sync::Arc;

use batteries::prelude::Dot2D;
use cgmath::{Point2, Vector2};

use crate::{
    document::{ArtboardId, LayerId},
    editor_state::BrushProperties,
    renderer::render_context::RenderContext,
};

/// Controller events are created to add an indirection so the events can be replayed.
/// This is intended to build the undo/redo functionality in the future.
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    BrushPoint {
        dot: Dot2D,
    },
    CameraMove {
        world_delta: cgmath::Vector2<f32>,
    },
    CameraZoom {
        delta: f32,
        /// cursor position used as zoom anchor
        screen: cgmath::Point2<f32>,
    },
    SelectArtboard(ArtboardId),
    SelectLayer(ArtboardId, LayerId),
    ClearSelection,
    MoveLayer {
        layer: LayerId,
        world_delta: Vector2<f32>,
    },
    MoveArtboard {
        artboard: ArtboardId,
        world_delta: Vector2<f32>,
    },
    ClearLayer {
        layer: LayerId,
    },
    ClearCanvas,
    UpdateBrush(BrushProperties),
    StrokeStart,
    StrokeEnd,
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
        /// Zoom anchor
        screen: Point2<f32>,
    },
    ClearCanvas,
    UpdateBrush(BrushProperties),
    StrokeStart,
    StrokeEnd,
}
