use crate::{
    prelude::*,
    renderer::{egui_context::EguiContext, render_context::RenderContext},
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::RwLock,
    thread::sleep,
    time::Duration,
};
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub struct WindowResource(pub Arc<winit::window::Window>);
impl Resource for WindowResource {}

pub struct App {
    brush_controller: BrushController,
    camera_controller: CameraController,
    resources: HashMap<TypeId, Arc<RwLock<dyn Any + Send + Sync>>>,
    startup_systems: Vec<Box<dyn System>>,
    update_systems: Vec<Box<dyn System>>,
    proxy: EventLoopProxy<CustomEvent>,
}

impl App {
    pub fn new(event_loop_proxy: EventLoopProxy<CustomEvent>) -> Self {
        let event_sender = EventSender::new(event_loop_proxy.clone());
        Self {
            brush_controller: BrushController::new(event_sender.clone()),
            camera_controller: CameraController::new(event_sender),
            resources: HashMap::new(),
            startup_systems: vec![],
            update_systems: vec![],
            proxy: event_loop_proxy,
        }
    }

    fn run_startup_systems(&self) {
        for system in &self.startup_systems {
            system.run(self);
        }
    }

    fn run_update_systems(&self) {
        for system in &self.update_systems {
            system.run(self);
        }
    }
}

impl ResourceContext for App {
    fn read<T: Resource>(&self) -> Option<Res<'_, T>> {
        let guard = self.resources.get(&TypeId::of::<T>())?.read().ok()?;

        Some(Res::new(guard))
    }

    fn write<T: Resource>(&self) -> Option<ResMut<'_, T>> {
        let guard = self.resources.get(&TypeId::of::<T>())?.write().ok()?;

        Some(ResMut::new(guard))
    }

    fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
        self.resources
            .insert(TypeId::of::<T>(), Arc::new(RwLock::new(resource)));

        self
    }
}

impl SystemRegistry for App {
    fn add_system(&mut self, schedule: Schedule, system: impl System + 'static) -> &mut Self {
        match schedule {
            Schedule::Startup => self.startup_systems.push(Box::new(system)),
            Schedule::Update => self.update_systems.push(Box::new(system)),
        }

        self
    }
}

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.read::<WindowResource>().is_none() {
            // updated by wasm window attributes
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
            let render_context = pollster::block_on(RenderContext::new(window.clone()));
            let egui_context = EguiContext::new(window.clone(), &render_context);
            let app_state = State::new();

            self.insert_resource(render_context)
                .insert_resource(egui_context)
                .insert_resource(app_state)
                .insert_resource(WindowResource(window));
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::ClearCanvas => {
                if let Some(state) = &mut self.write::<State>() {
                    state.clear_canvas();
                    if let Some(window_res) = &self.read::<WindowResource>() {
                        window_res.0.request_redraw();
                    }
                }
            }
            CustomEvent::CameraMove { position } => {
                if let Some(window_res) = &self.read::<WindowResource>() {
                    let window_size = window_res.0.inner_size();

                    if let Some(state) = &mut self.write::<State>() {
                        let world_translation = screen_to_world_position(
                            position,
                            #[allow(clippy::cast_precision_loss)]
                            (window_size.width as f32, window_size.height as f32),
                        );

                        state.update_camera(Some(CameraTransform {
                            translation: Some(clamp::clamp_point(world_translation)),
                            ..Default::default()
                        }));
                        window_res.0.request_redraw();
                    }
                }
            }
            CustomEvent::CameraZoom { delta } => {
                if let Some(state) = &mut self.write::<State>() {
                    state.update_camera(Some(CameraTransform {
                        scale_delta: Some(delta),
                        ..Default::default()
                    }));
                    if let Some(window) = &self.read::<WindowResource>() {
                        window.0.request_redraw();
                    }
                }
            }
            CustomEvent::BrushPoint { dot } => {
                if let Some(window) = &self.read::<WindowResource>() {
                    let window_size = window.0.inner_size();

                    let brush_position = world_to_ndc(
                        dot.position,
                        #[allow(clippy::cast_precision_loss)]
                        (window_size.width as f32, window_size.height as f32),
                    );
                    let clamped_position = clamp::clamp_point(brush_position);

                    if let Some(state) = &mut self.write::<State>() {
                        state.update_paint(&Dot2D {
                            position: clamped_position,
                            radius: dot.radius,
                        });
                        state.paint_to_texture();
                        window.0.request_redraw();
                    }
                }
            }
            CustomEvent::UpdateBrushColor(color) => {
                if let Some(state) = &mut self.write::<State>() {
                    state.update_brush_color(color);
                    if let Some(window) = &self.read::<WindowResource>() {
                        window.0.request_redraw();
                    }
                }
            }
            // this is useful for syncing UI with tools eg. UI needs to show a larger brush pointer when zoomed in
            CustomEvent::_UiUpdate => {}
            CustomEvent::CanvasCreated { state } => {
                self.insert_resource(*state);
                if let Some(window) = &self.read::<WindowResource>() {
                    window.0.request_redraw();
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
        // Pass events to egui first, before any other processing
        if let (Some(mut egui_ctx), Some(window)) =
            (self.write::<EguiContext>(), self.read::<WindowResource>())
        {
            let event_response = egui_ctx.egui_state.on_window_event(&window.0, &event);
            if event_response.consumed {
                // Egui consumed the event, don't process further
                return;
            }
        }

        // Run update systems before acquiring State lock
        if matches!(event, WindowEvent::RedrawRequested) {
            self.run_update_systems();
        }

        // Check if State exists before processing events
        let Some(app_state) = &mut self.write::<State>() else {
            // if there's no app_state, the window might not have been initialized
            // no need to start processing events yet
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(mut render_ctx) = self.write::<RenderContext>() {
                    render_ctx.reconfigure(size);
                }
            }
            WindowEvent::RedrawRequested => {
                match app_state.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        // re-configure to the same window size as the one just lost
                        if let Some(window_res) = self.read::<WindowResource>() {
                            let size = window_res.0.inner_size();
                            app_state.resize(size.width, size.height);
                        }
                    }
                    Err(e) => {
                        log::error!("Unable to render to display {e}");
                    }
                }

                // Cap framerate
                sleep(Duration::from_millis(5));
                let now = Instant::now();
                app_state.last_render = now;
                if let Some(window_res) = self.read::<WindowResource>() {
                    window_res.0.request_redraw();
                }
            }
            event => {
                // Pass events to UI first, return early if consumed
                if app_state.handle_ui_event(&event) {
                    return;
                }

                // TODO: decouple brush and camera controllers
                // self.brush_controller.process_event(&event);
                // self.camera_controller.process_event(&event);

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
