//! Layer-context handler (§3.3): the stroke machinery (moved from the retired
//! `BrushController`) plus the cmd+drag move-layer interaction.

use batteries::prelude::{Dot2D, PointProcessor, StrokeDot2D};
use cgmath::{EuclideanSpace, Point2};
use winit::keyboard::KeyCode;

use crate::{
    events::ControllerEvent,
    input::{
        dispatch::{ContextHandler, DispatchEnv, DragTracker, Handled, InputAction},
        selection::SelectionCtx,
    },
};

const BRUSH_STEP_SIZE: f32 = 1.0;

pub struct LayerContextHandler {
    move_drag: DragTracker,
    point_processor: PointProcessor,
    /// Last emitted brush point (screen px), flushed with `is_last` on
    /// pointer-up so the stroke tail is stamped.
    brush_position: Point2<f32>,
}

impl LayerContextHandler {
    pub fn new() -> Self {
        Self {
            move_drag: DragTracker::default(),
            point_processor: PointProcessor::new(BRUSH_STEP_SIZE),
            brush_position: Point2::origin(),
        }
    }

    fn emit_points(&mut self, dot: StrokeDot2D, env: &DispatchEnv) {
        let points = self.point_processor.process_point(dot);
        for point in &points {
            env.sender.send(ControllerEvent::BrushPoint {
                dot: Dot2D {
                    position: *point,
                    radius: env.brush_size,
                },
            });
        }
        if let Some(last_point) = points.last() {
            self.brush_position = *last_point;
        }
    }
}

impl ContextHandler for LayerContextHandler {
    fn handle(&mut self, ctx: SelectionCtx, action: &InputAction, env: &DispatchEnv) -> Handled {
        let SelectionCtx::Layer(artboard_id, layer_id) = ctx else {
            return Handled::No;
        };

        match *action {
            InputAction::PointerDown { screen } => {
                if env.modifiers.super_key() {
                    self.move_drag.begin(screen);
                    return Handled::Yes;
                }
                // inside own artboard → draw; outside → bubble
                let world = env.camera.screen_to_world(screen);
                let inside = env
                    .doc
                    .artboard(artboard_id)
                    .is_some_and(|artboard| artboard.contains(world));
                if inside {
                    env.sender.send(ControllerEvent::StrokeStart);
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::PointerMove { screen } => {
                if env.modifiers.super_key() {
                    if let Some(screen_delta) = self.move_drag.step(screen) {
                        env.sender.send(ControllerEvent::MoveLayer {
                            layer: layer_id,
                            world_delta: env.camera.screen_delta_to_world(screen_delta),
                        });
                        return Handled::Yes;
                    }
                    return Handled::No;
                }
                if env.stroke_active {
                    self.emit_points(
                        StrokeDot2D {
                            position: screen,
                            radius: env.brush_size,
                            is_last: false,
                        },
                        env,
                    );
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::PointerUp { .. } => {
                if self.move_drag.end() {
                    return Handled::Yes;
                }
                if env.stroke_active {
                    // stamp the stroke tail, then commit
                    self.emit_points(
                        StrokeDot2D {
                            position: self.brush_position,
                            radius: env.brush_size,
                            is_last: true,
                        },
                        env,
                    );
                    self.point_processor.clear();
                    env.sender.send(ControllerEvent::StrokeEnd);
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::Key { code, pressed } => {
                if pressed && code == KeyCode::KeyR && env.modifiers.super_key() {
                    env.sender
                        .send(ControllerEvent::ClearLayer { layer: layer_id });
                    return Handled::Yes;
                }
                Handled::No
            }
            InputAction::Scroll { .. } => Handled::No,
        }
    }
}
