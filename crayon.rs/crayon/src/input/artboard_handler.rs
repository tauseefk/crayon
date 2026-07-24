use crate::{
    events::ControllerEvent,
    input::{
        dispatch::{ContextHandler, DispatchEnv, DragTracker, Handled, InputAction},
        selection::SelectionCtx,
    },
};

pub struct ArtboardContextHandler {
    move_drag: DragTracker,
}

impl ArtboardContextHandler {
    pub fn new() -> Self {
        Self {
            move_drag: DragTracker::default(),
        }
    }
}

impl ContextHandler for ArtboardContextHandler {
    fn handle(&mut self, ctx: SelectionCtx, action: &InputAction, env: &DispatchEnv) -> Handled {
        let SelectionCtx::Artboard(artboard_id) = ctx else {
            return Handled::No;
        };

        match *action {
            InputAction::PointerDown { screen } => {
                if env.modifiers.super_key() {
                    self.move_drag.begin(screen);
                    return Handled::Yes;
                }
                let world_pos = env.camera.screen_to_world(screen);
                let Some(artboard) = env.doc.artboard(artboard_id) else {
                    return Handled::No;
                };
                if !artboard.contains(world_pos) {
                    return Handled::No;
                }

                if let Some(layer) = artboard.layers.last() {
                    env.sender
                        .send(ControllerEvent::SelectLayer(artboard_id, layer.id));
                    env.sender.send(ControllerEvent::StrokeStart);
                }
                Handled::Yes
            }
            InputAction::PointerMove { screen } => {
                if env.modifiers.super_key()
                    && let Some(screen_delta) = self.move_drag.step(screen)
                {
                    env.sender.send(ControllerEvent::MoveArtboard {
                        artboard: artboard_id,
                        world_delta: env.camera.screen_delta_to_world(screen_delta),
                    });
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::PointerUp { .. } => {
                if self.move_drag.end() {
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::Scroll { .. } | InputAction::Key { .. } => Handled::No,
        }
    }
}
