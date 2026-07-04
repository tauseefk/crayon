use winit::event::WindowEvent;

use crate::{
    brush_controller::BrushController, camera_controller::CameraController,
    event_sender::EventSender, resource::Resource,
};

pub struct InputSystem {
    brush_controller: BrushController,
    camera_controller: CameraController,
    is_super_pressed: bool,
}

impl InputSystem {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            brush_controller: BrushController::new(event_sender.clone()),
            camera_controller: CameraController::new(event_sender),
            is_super_pressed: false,
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent, brush_size: f32) {
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.is_super_pressed = modifiers.state().super_key();
        }

        self.brush_controller
            .process_event(event, brush_size, self.is_super_pressed);
        self.camera_controller
            .process_event(event, self.is_super_pressed);
    }
}

impl Resource for InputSystem {}
