use crate::events::ControllerEvent;
use crate::{events::CustomEvent, resource::Resource};

impl From<ControllerEvent> for CustomEvent {
    fn from(event: ControllerEvent) -> Self {
        match event {
            ControllerEvent::BrushPoint { dot } => CustomEvent::BrushPoint { dot },
            ControllerEvent::CameraMove { world_delta } => CustomEvent::CameraMove { world_delta },
            ControllerEvent::CameraZoom { delta } => CustomEvent::CameraZoom { delta },
            ControllerEvent::SelectArtboard(artboard) => CustomEvent::SelectArtboard(artboard),
            ControllerEvent::SelectLayer(artboard, layer) => {
                CustomEvent::SelectLayer(artboard, layer)
            }
            ControllerEvent::ClearSelection => CustomEvent::ClearSelection,
            ControllerEvent::MoveLayer { layer, world_delta } => {
                CustomEvent::MoveLayer { layer, world_delta }
            }
            ControllerEvent::MoveArtboard {
                artboard,
                world_delta,
            } => CustomEvent::MoveArtboard {
                artboard,
                world_delta,
            },
            ControllerEvent::ClearLayer { layer } => CustomEvent::ClearLayer { layer },
            ControllerEvent::AddArtboard => CustomEvent::AddArtboard,
            ControllerEvent::DeleteArtboard(artboard) => CustomEvent::DeleteArtboard(artboard),
            ControllerEvent::AddLayer(artboard) => CustomEvent::AddLayer(artboard),
            ControllerEvent::DeleteLayer(layer) => CustomEvent::DeleteLayer(layer),
            ControllerEvent::ToggleLayerVisibility(layer) => {
                CustomEvent::ToggleLayerVisibility(layer)
            }
            ControllerEvent::UpdateBrush(properties) => CustomEvent::UpdateBrush(properties),
            ControllerEvent::StrokeStart => CustomEvent::StrokeStart,
            ControllerEvent::StrokeEnd => CustomEvent::StrokeEnd,
        }
    }
}

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
                    let _ = proxy_clone.send_event(event.into());
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

    /// Test-only sender whose events are captured instead of relayed to the
    /// event loop: the mpsc pair is created without the relay thread, so the
    /// returned receiver holds everything `send` produces.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub fn capturing() -> (Self, std::sync::mpsc::Receiver<ControllerEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Self { channel: tx }, rx)
    }

    /// This relays the controller events to appropriate channels
    ///
    /// Non-WASM targets have an added level of indirection of an mpsc channel which allows storing the events and replaying.
    /// WASM target directly passes the event to the event loop proxy.
    pub fn send(&self, event: ControllerEvent) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self.proxy.send_event(event.into());
        }

        // directly pass the event along for desktop environment
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = self.channel.send(event);
        }
    }
}

impl Resource for EventSender {}
