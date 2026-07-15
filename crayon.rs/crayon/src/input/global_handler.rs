//! Global-context handler (§3.3), always the outermost frame: camera pan and
//! zoom (moved from the retired `CameraController`), artboard hit-select on
//! click, and the stroke-end safety net for strokes whose layer frame was
//! popped mid-drag.

use crate::{
    events::ControllerEvent,
    input::{
        dispatch::{ContextHandler, DispatchEnv, DragTracker, Handled, InputAction},
        selection::SelectionCtx,
    },
    utils::zoom,
};

pub struct GlobalContextHandler {
    pan_drag: DragTracker,
}

impl GlobalContextHandler {
    pub fn new() -> Self {
        Self {
            pan_drag: DragTracker::default(),
        }
    }
}

impl ContextHandler for GlobalContextHandler {
    fn handle(&mut self, _ctx: SelectionCtx, action: &InputAction, env: &DispatchEnv) -> Handled {
        match *action {
            InputAction::PointerDown { screen } => {
                if env.modifiers.super_key() {
                    self.pan_drag.begin(screen);
                    return Handled::Yes;
                }
                // reverse-draw-order hit test: hit selects, miss clears
                let world = env.camera.screen_to_world(screen);
                match env.doc.hit_test(world) {
                    Some(artboard) => env.sender.send(ControllerEvent::SelectArtboard(artboard)),
                    None => env.sender.send(ControllerEvent::ClearSelection),
                }
                Handled::Yes
            }
            InputAction::PointerMove { screen } => {
                if env.modifiers.super_key()
                    && let Some(screen_delta) = self.pan_drag.step(screen)
                {
                    env.sender.send(ControllerEvent::CameraMove {
                        world_delta: env.camera.screen_delta_to_world(screen_delta),
                    });
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::PointerUp { .. } => {
                if self.pan_drag.end() {
                    return Handled::Yes;
                }
                // A stroke whose layer frame was popped mid-drag (Esc) still
                // commits on release.
                if env.stroke_active {
                    env.sender.send(ControllerEvent::StrokeEnd);
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::Scroll { delta, .. } => {
                if !env.modifiers.super_key() {
                    return Handled::No;
                }
                let zoom_delta = zoom::get_zoom_delta(delta);
                if zoom_delta != 0.0 {
                    env.sender.send(ControllerEvent::CameraZoom {
                        delta: zoom_delta,
                    });
                }
                Handled::Yes
            }
            // Esc lives in `app.rs` (it needs the event loop to exit); no
            // other global keys remain — global clear-canvas is retired.
            InputAction::Key { .. } => Handled::No,
        }
    }
}
