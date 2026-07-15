//! Dispatch model (multi-artboard.md §3.2): raw winit events are normalized
//! once into `InputAction`s, then bubbled innermost → outermost through the
//! selection stack until a handler claims them.

use cgmath::{Point2, Vector2};
use winit::keyboard::{KeyCode, ModifiersState};

use crate::{
    document::Document, event_sender::EventSender, input::selection::SelectionCtx,
    input::selection::SelectionStack, renderer::camera::Camera2D,
};

#[derive(Clone, Copy, Debug)]
pub enum InputAction {
    PointerDown {
        screen: Point2<f32>,
    },
    PointerMove {
        screen: Point2<f32>,
    },
    PointerUp {
        /// Part of the §3.2 action shape; no handler reads it yet.
        #[allow(dead_code)]
        screen: Point2<f32>,
    },
    Scroll {
        delta: f32,
        /// Part of the §3.2 action shape; consumed when zoom-at-cursor lands.
        #[allow(dead_code)]
        screen: Point2<f32>,
    },
    Key {
        code: KeyCode,
        pressed: bool,
    },
}

/// Read-only view of the app the handlers dispatch against. Handlers never
/// mutate state directly — they send `ControllerEvent`s (lock discipline, §6).
pub struct DispatchEnv<'a> {
    /// Stamped by `InputSystem` from its `ModifiersChanged` tracking before
    /// dispatch; the value the caller supplies is overwritten.
    pub modifiers: ModifiersState,
    /// Snapshot for `screen_to_world`.
    pub camera: Camera2D,
    /// Hit-testing, rects.
    pub doc: &'a Document,
    pub selection: &'a SelectionStack,
    pub brush_size: f32,
    /// From `StrokeState` — a stroke is in flight, whoever claimed the
    /// pointer-down. Lets the layer handler feed points to strokes started by
    /// the artboard handler's select-and-draw path.
    pub stroke_active: bool,
    pub sender: &'a EventSender,
}

pub enum Handled {
    Yes,
    No,
}

pub trait ContextHandler {
    fn handle(&mut self, ctx: SelectionCtx, action: &InputAction, env: &DispatchEnv) -> Handled;
}

/// Cmd+drag state shared by all three handlers: begun on pointer-down,
/// stepped on pointer-move (yielding the screen-px delta since the last
/// step), ended on pointer-up.
#[derive(Default)]
pub struct DragTracker {
    last: Option<Point2<f32>>,
}

impl DragTracker {
    pub fn begin(&mut self, screen: Point2<f32>) {
        self.last = Some(screen);
    }

    pub fn step(&mut self, screen: Point2<f32>) -> Option<Vector2<f32>> {
        let last = self.last?;
        self.last = Some(screen);
        Some(screen - last)
    }

    /// Returns whether a drag was active.
    pub fn end(&mut self) -> bool {
        self.last.take().is_some()
    }
}
