use std::sync::Arc;

use batteries::prelude::Dot2D;
use cgmath::Vector2;

use crate::{
    document::{ArtboardId, LayerId, loader::LoadedDocument},
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
        /// Drag deltas are converted where the semantics live:
        /// `world_delta = screen_delta / camera.scale` (§3.3).
        world_delta: Vector2<f32>,
    },
    CameraZoom {
        delta: f32,
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
    AddArtboard,
    DeleteArtboard(ArtboardId),
    AddLayer(ArtboardId),
    DeleteLayer(LayerId),
    ToggleLayerVisibility(LayerId),
    /// Run the platform file dialog and load the picked document (§1.9).
    OpenDocument,
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
    /// Async fetch completion, only sent on the WASM target (§1.8). Arrives
    /// twice per load: thumbhash placeholders first, real PNG content second
    /// (§1.6). Each delivery atomically replaces the document and every layer
    /// texture.
    #[allow(dead_code)]
    DocumentLoaded(Box<LoadedDocument>),
    CameraMove {
        world_delta: Vector2<f32>,
    },
    CameraZoom {
        delta: f32,
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
    AddArtboard,
    DeleteArtboard(ArtboardId),
    AddLayer(ArtboardId),
    DeleteLayer(LayerId),
    ToggleLayerVisibility(LayerId),
    OpenDocument,
    UpdateBrush(BrushProperties),
    StrokeStart,
    StrokeEnd,
}
