use crate::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub struct App {
    proxy: EventLoopProxy<CustomEvent>,
    state: Option<State>,
    window: Option<Arc<winit::window::Window>>,
    camera_controller: CameraController,
    brush_controller: BrushController,
    last_render: Instant,
}

impl App {
    pub fn new(event_loop_proxy: EventLoopProxy<CustomEvent>) -> Self {
        let event_sender = EventSender::new(event_loop_proxy.clone());
        Self {
            brush_controller: BrushController::new(event_sender.clone()),
            camera_controller: CameraController::new(event_sender),
            last_render: Instant::now(),
            proxy: event_loop_proxy,
            state: None,
            window: None,
        }
    }
}

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes()
            .with_title("Crayon")
            .with_inner_size(LogicalSize::new(WINDOW_SIZE.0, WINDOW_SIZE.1));

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window = Some(window.clone());

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let (proxy, Some(window)) = (self.proxy.clone(), self.window.clone()) {
                let event_sender = EventSender::new(proxy);
                let state =
                    futures::executor::block_on(State::new(window.clone(), event_sender)).unwrap();

                self.state = Some(state);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let window_for_wasm = window.clone();
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    let event_sender = EventSender::new(proxy.clone());
                    let state = State::new(window_for_wasm, event_sender)
                        .await
                        .expect("Unable to create canvas!!!");
                    assert!(
                        proxy
                            .send_event(CustomEvent::CanvasCreated {
                                state: Box::new(state)
                            })
                            .is_ok()
                    );
                });
            }
        }

        window.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::ClearCanvas => {
                if let Some(state) = &mut self.state {
                    state.clear_canvas();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            CustomEvent::CameraMove { position } => {
                if let Some(window) = &self.window {
                    let window_size = window.inner_size();

                    if let Some(state) = &mut self.state {
                        let world_translation = screen_to_world_position(
                            position,
                            #[allow(clippy::cast_precision_loss)]
                            (window_size.width as f32, window_size.height as f32),
                        );

                        state.update_camera(Some(CameraTransform {
                            translation: Some(clamp::clamp_point(world_translation)),
                            ..Default::default()
                        }));
                        window.request_redraw();
                    }
                }
            }
            CustomEvent::CameraZoom { delta } => {
                if let Some(state) = &mut self.state {
                    state.update_camera(Some(CameraTransform {
                        scale_delta: Some(delta),
                        ..Default::default()
                    }));
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            CustomEvent::BrushPoint { dot } => {
                if let Some(window) = &self.window {
                    let window_size = window.inner_size();

                    let brush_position = world_to_ndc(
                        dot.position,
                        #[allow(clippy::cast_precision_loss)]
                        (window_size.width as f32, window_size.height as f32),
                    );
                    let clamped_position = clamp::clamp_point(brush_position);

                    if let Some(state) = &mut self.state {
                        state.update_paint(&Dot2D {
                            position: clamped_position,
                            radius: dot.radius,
                        });
                        state.paint_to_texture();
                        window.request_redraw();
                    }
                }
            }
            CustomEvent::UpdateBrushColor(color) => {
                if let Some(state) = &mut self.state {
                    state.update_brush_color(color);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            // this is useful for syncing UI with tools eg. UI needs to show a larger brush pointer when zoomed in
            CustomEvent::_UiUpdate => {}
            CustomEvent::CanvasCreated { state } => {
                self.state = Some(*state);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(app_state) = &mut self.state else {
            // if there's no app_state, the window might not have been initialized
            // no need to start processing events yet
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => app_state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match app_state.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        // re-configure to the same window size as the one just lost
                        let size = app_state.get_window_size();
                        app_state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render to display {e}");
                    }
                };

                // Cap framerate
                let now = Instant::now();
                if now.duration_since(self.last_render).as_millis() >= 5 {
                    self.last_render = now;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            event => {
                // Pass events to UI first, return early if consumed
                if app_state.handle_ui_event(&event) {
                    return;
                }

                self.brush_controller.process_event(&event);
                self.camera_controller.process_event(&event);

                if let WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state: key_state,
                            ..
                        },
                    ..
                } = event
                    && let (KeyCode::Escape, true) = (code, key_state.is_pressed())
                {
                    event_loop.exit();
                }
            }
        }
    }
}
