use cgmath::{Point2, Vector2};
use winit::keyboard::{KeyCode, ModifiersState};

use crate::{
    document::Document,
    event_sender::EventSender,
    input::selection::{SelectionCtx, SelectionStack},
    renderer::camera::Camera2D,
};

pub enum Handled {
    Yes,
    No,
}

pub enum InputAction {
    PointerDown { screen: Point2<f32> },
    PointerMove { screen: Point2<f32> },
    PointerUp { screen: Point2<f32> },
    Scroll { delta: f32, screen: Point2<f32> },
    Key { code: KeyCode, pressed: bool },
}

pub struct DispatchEnv<'ev> {
    pub modifiers: ModifiersState,
    pub camera: Camera2D,
    pub doc: &'ev Document,
    pub selection: &'ev SelectionStack,
    pub brush_size: f32,
    pub stroke_active: bool,
    pub sender: &'ev EventSender,
}

pub trait ContextHandler {
    fn handle(&mut self, ctx: SelectionCtx, action: &InputAction, env: &DispatchEnv) -> Handled;
}

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

    /// Returns whether the drag was active.
    pub fn end(&mut self) -> bool {
        self.last.take().is_some()
    }
}
