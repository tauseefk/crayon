use crate::prelude::*;

pub struct BrushController {
    event_sender: EventSender,
    is_mouse_down: bool,
    is_dragging: bool,
    is_disabled: bool,
    cursor_size: f32,
    cursor_position: cgmath::Point2<f32>,
    point_processor: PointProcessor,
}

impl BrushController {
    pub fn new(event_sender: EventSender) -> Self {
        let point_processor = PointProcessor::new(100., 1., 1.);
        BrushController {
            event_sender,
            is_dragging: false,
            is_mouse_down: false,
            is_disabled: false,
            cursor_size: 2.0,
            cursor_position: Point2::origin(),
            point_processor,
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
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
                    position: cgmath::Point2::new(position.x as f32, position.y as f32),
                    radius: self.cursor_size,
                    is_last: false,
                });

                for point in points.iter() {
                    let _ = self
                        .event_sender
                        .send(ControllerEvent::BrushPoint { position: *point });
                }

                // Update internal position to last point for usage in mouse up
                if let Some(last_point) = points.last() {
                    self.cursor_position = *last_point;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == MouseButton::Left {
                    let was_mouse_down = self.is_mouse_down;
                    self.is_mouse_down = *state == ElementState::Pressed;

                    if was_mouse_down && !self.is_mouse_down {
                        // Process final point with is_last=true for stroke end
                        let final_points = self.point_processor.process_point(StrokeDot2D {
                            position: self.cursor_position,
                            radius: self.cursor_size,
                            is_last: true,
                        });

                        for point in final_points {
                            let _ = self
                                .event_sender
                                .send(ControllerEvent::BrushPoint { position: point });
                        }

                        self.point_processor.clear();
                    }
                }
            }
            _ => {}
        }
    }
}
