use crate::prelude::*;
use crate::resource::Resource;

pub struct InputSystem {
    brush_controller: BrushController,
    camera_controller: CameraController,
}

impl InputSystem {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            brush_controller: BrushController::new(event_sender.clone()),
            camera_controller: CameraController::new(event_sender),
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        self.brush_controller.process_event(event);
        self.camera_controller.process_event(event);
    }
}

impl Resource for InputSystem {}
