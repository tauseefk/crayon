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
                let world_pos = env.camera.screen_to_world(screen);
                match env.doc.hit_test(world_pos) {
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
                if env.stroke_active {
                    env.sender.send(ControllerEvent::StrokeEnd);
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::Scroll { delta, screen } => {
                if !env.modifiers.super_key() {
                    return Handled::No;
                }

                let zoom_delta = zoom::get_zoom_delta(delta);
                if zoom_delta != 0.0 {
                    env.sender.send(ControllerEvent::CameraZoom {
                        delta: zoom_delta,
                        screen,
                    });
                }
                Handled::Yes
            }
            InputAction::Key { .. } => Handled::No,
        }
    }
}
