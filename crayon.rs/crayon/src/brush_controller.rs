use batteries::prelude::{Dot2D, PointProcessor, StrokeDot2D};
use cgmath::{EuclideanSpace, Point2};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    event_sender::EventSender, events::ControllerEvent, renderer::brush::DEFAULT_BRUSH_SIZE,
};

const BRUSH_STEP_SIZE: f32 = 1.0;

pub struct BrushController {
    event_sender: EventSender,
    is_mouse_down: bool,
    is_dragging: bool,
    is_disabled: bool,
    brush_size: f32,
    brush_position: cgmath::Point2<f32>,
    point_processor: PointProcessor,
}

impl BrushController {
    pub fn new(event_sender: EventSender) -> Self {
        let point_processor = PointProcessor::new(BRUSH_STEP_SIZE);
        BrushController {
            event_sender,
            is_dragging: false,
            is_mouse_down: false,
            is_disabled: false,
            brush_size: DEFAULT_BRUSH_SIZE,
            brush_position: Point2::origin(),
            point_processor,
        }
    }

    // TODO: threading brush_size isn't the cleanest approach
    pub fn process_event(&mut self, event: &WindowEvent, brush_size: f32) {
        self.brush_size = brush_size;
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::SuperLeft)
                    || event.physical_key == PhysicalKey::Code(KeyCode::SuperRight)
                {
                    self.is_disabled = event.state == ElementState::Pressed;
                }

                if self.is_disabled
                    && event.physical_key == PhysicalKey::Code(KeyCode::KeyR)
                    && event.state.is_pressed()
                {
                    self.event_sender.send(ControllerEvent::ClearCanvas);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.is_dragging = self.is_mouse_down;

                if !self.is_dragging || self.is_disabled {
                    return;
                }

                let points = self.point_processor.process_point(StrokeDot2D {
                    #[allow(clippy::cast_possible_truncation)]
                    position: cgmath::Point2::new(position.x as f32, position.y as f32),
                    radius: self.brush_size,
                    is_last: false,
                });

                for point in &points {
                    self.event_sender.send(ControllerEvent::BrushPoint {
                        dot: Dot2D {
                            position: *point,
                            radius: self.brush_size,
                        },
                    });
                }

                // Update internal position to last point for usage in mouse up
                if let Some(last_point) = points.last() {
                    self.brush_position = *last_point;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    let was_mouse_down = self.is_mouse_down;
                    self.is_mouse_down = *state == ElementState::Pressed;

                    if was_mouse_down && !self.is_mouse_down {
                        // Process final point with is_last=true for stroke end
                        let final_points = self.point_processor.process_point(StrokeDot2D {
                            position: self.brush_position,
                            radius: self.brush_size,
                            is_last: true,
                        });

                        for point in final_points {
                            self.event_sender.send(ControllerEvent::BrushPoint {
                                dot: Dot2D {
                                    position: point,
                                    radius: self.brush_size,
                                },
                            });
                        }

                        self.point_processor.clear();
                    }
                }
            }
            _ => {}
        }
    }
}
