use crate::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
pub type ControllerEventSender = std::sync::mpsc::Sender<ControllerEvent>;

#[derive(Clone)]
pub struct EventSender {
    #[cfg(target_arch = "wasm32")]
    proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
    #[cfg(not(target_arch = "wasm32"))]
    channel: ControllerEventSender,
}

impl EventSender {
    pub fn new(event_loop_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (tx, rx) = std::sync::mpsc::channel::<ControllerEvent>();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let proxy_clone = event_loop_proxy.clone();
            std::thread::spawn(move || {
                while let Ok(event) = rx.recv() {
                    match event {
                        ControllerEvent::BrushPoint { dot } => {
                            let _ = proxy_clone.send_event(CustomEvent::BrushPoint { dot });
                        }
                        ControllerEvent::CameraMove { position } => {
                            let _ = proxy_clone.send_event(CustomEvent::CameraMove { position });
                        }
                        ControllerEvent::CameraZoom { delta, .. } => {
                            let _ = proxy_clone.send_event(CustomEvent::CameraZoom { delta });
                        }
                        ControllerEvent::ClearCanvas => {
                            let _ = proxy_clone.send_event(CustomEvent::ClearCanvas);
                        }
                    }
                }
            });
        }

        Self {
            #[cfg(target_arch = "wasm32")]
            proxy: event_loop_proxy,
            #[cfg(not(target_arch = "wasm32"))]
            channel: tx,
        }
    }

    /// This relays the controller events to appropriate channels
    ///
    /// Non-WASM targets have an added level of indirection of an mpsc channel which allows storing the events and replaying.
    /// WASM target directly passes the event to the event loop proxy.
    pub fn send(&self, event: ControllerEvent) {
        #[cfg(target_arch = "wasm32")]
        {
            match event {
                ControllerEvent::BrushPoint { dot } => {
                    let _ = self.proxy.send_event(CustomEvent::BrushPoint { dot });
                }
                ControllerEvent::CameraMove { position } => {
                    let _ = self.proxy.send_event(CustomEvent::CameraMove { position });
                }
                ControllerEvent::CameraZoom { delta, .. } => {
                    let _ = self.proxy.send_event(CustomEvent::CameraZoom { delta });
                }
                ControllerEvent::ClearCanvas => {
                    let _ = self.proxy.send_event(CustomEvent::ClearCanvas);
                }
            }
        }

        // directly pass the event along for desktop environment
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.channel.send(event);
        }
    }
}
