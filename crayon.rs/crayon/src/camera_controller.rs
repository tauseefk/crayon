use crate::prelude::*;

/// This represents the camera controller that's used to control zooming and panning of the drawing canvas.
///
pub struct CameraController {
    event_sender: EventSender,
    is_mouse_down: bool,
    is_super_pressed: bool,
    is_dragging: bool,
    /// Translation offset for camera
    /// persists between multiple panning operations
    translation_offset: Point2<f32>,
    /// Cursor position during a drag operation
    cursor_position: Point2<f32>,
    /// Initialized to `cursor_position` on mouse down
    cursor_start_position: Point2<f32>,
}

impl CameraController {
    pub fn new(event_sender: EventSender) -> Self {
        CameraController {
            event_sender,
            is_dragging: false,
            is_super_pressed: false,
            is_mouse_down: false,
            cursor_position: Point2::origin(),
            translation_offset: Point2::origin(),
            cursor_start_position: Point2::origin(),
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::SuperLeft)
                    || event.physical_key == PhysicalKey::Code(KeyCode::SuperRight)
                {
                    self.is_super_pressed = event.state == ElementState::Pressed;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // if super isn't pressed camera control should be disabled
                if !self.is_super_pressed {
                    return;
                }

                let zoom_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => zoom::get_zoom_delta(*y),
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) =>
                    {
                        #[allow(clippy::cast_possible_truncation)]
                        zoom::get_zoom_delta(*y as f32)
                    }
                };

                if zoom_delta == 0.0 {
                    return;
                }

                self.event_sender.send(ControllerEvent::CameraZoom {
                    delta: zoom_delta,
                    _position: self.cursor_position,
                });
            }
            WindowEvent::CursorMoved { position, .. } => {
                // if super isn't pressed camera control should be disabled
                if !self.is_super_pressed {
                    return;
                }

                self.cursor_position = Point2::new(position.x as f32, position.y as f32);

                if self.is_mouse_down && self.is_super_pressed {
                    self.is_dragging = true;

                    // keep offset local during drag operation
                    let total_offset = self.translation_offset
                        + (self.cursor_position - self.cursor_start_position);

                    self.event_sender.send(ControllerEvent::CameraMove {
                        position: total_offset,
                    });
                } else {
                    self.is_dragging = false;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                // if super isn't pressed camera control should be disabled
                if !self.is_super_pressed {
                    return;
                }

                if *button == MouseButton::Left {
                    self.is_mouse_down = *state == ElementState::Pressed;
                    if self.is_mouse_down {
                        self.cursor_start_position = self.cursor_position;
                    } else {
                        self.is_dragging = false;

                        // persist translation offset after panning operation is finished
                        self.translation_offset +=
                            self.cursor_position - self.cursor_start_position;
                    }
                }
            }
            _ => {}
        }
    }
}
