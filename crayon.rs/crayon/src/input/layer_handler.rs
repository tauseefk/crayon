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

                let world_pos = env.camera.screen_to_world(screen);
                let is_point_inside = env
                    .doc
                    .artboard(artboard_id)
                    .is_some_and(|artboard| artboard.contains(world_pos));

                if is_point_inside {
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
                    self.emit_points(
                        StrokeDot2D {
                            position: self.brush_position,
                            radius: env.brush_size,
                            is_last: true,
                        },
                        env,
                    );
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
                return Handled::No;
            }
            InputAction::Scroll { .. } => Handled::No,
        }
    }
}
