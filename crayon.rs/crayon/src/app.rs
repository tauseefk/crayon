use crate::{
    prelude::*,
    renderer::{
        egui_context::EguiContext, frame_context::FrameContext, render_context::RenderContext,
    },
    resources::{
        brush_point_queue::BrushPointQueue, canvas_state::CanvasContext, input_system::InputSystem,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use std::thread::sleep;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::RwLock,
};

pub struct WindowResource(pub Arc<winit::window::Window>);
impl Resource for WindowResource {}

pub struct App {
    resources: HashMap<TypeId, Arc<RwLock<dyn Any + Send + Sync>>>,
    startup_systems: Vec<Box<dyn System>>,
    pre_update_systems: Vec<Box<dyn System>>,
    update_systems: Vec<Box<dyn System>>,
    post_update_systems: Vec<Box<dyn System>>,
    pub raw_point_count: u32,
    pub processed_point_count: u32,
}

impl App {
    pub fn new(event_loop_proxy: EventLoopProxy<CustomEvent>) -> Self {
        let event_sender = EventSender::new(event_loop_proxy.clone());

        let mut app = Self {
            resources: HashMap::new(),
            startup_systems: vec![],
            pre_update_systems: vec![],
            update_systems: vec![],
            post_update_systems: vec![],
            raw_point_count: 0,
            processed_point_count: 0,
        };

        // Store EventSender as a resource for UI widgets
        app.insert_resource(event_sender.clone());
        app.insert_resource(InputSystem::new(event_sender));

        app
    }

    fn _run_startup_systems(&self) {
        for system in &self.startup_systems {
            system.run(self);
        }
    }

    fn run_update_systems(&self) {
        // Run pre-update systems first
        for system in &self.pre_update_systems {
            system.run(self);
        }

        // Run main update systems
        for system in &self.update_systems {
            system.run(self);
        }

        // Run post-update systems last
        for system in &self.post_update_systems {
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
            Schedule::PreUpdate => self.pre_update_systems.push(Box::new(system)),
            Schedule::Update => self.update_systems.push(Box::new(system)),
            Schedule::PostUpdate => self.post_update_systems.push(Box::new(system)),
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
            let window_size = window.inner_size();
            let canvas_state =
                CanvasContext::new(&render_context, (window_size.width, window_size.height));
            let egui_context = EguiContext::new(window.clone(), &render_context);
            let app_state = State::new();

            self.insert_resource(render_context)
                .insert_resource(canvas_state)
                .insert_resource(egui_context)
                .insert_resource(app_state)
                .insert_resource(FrameContext::new())
                .insert_resource(WindowResource(window));

            self.run_update_systems();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::ClearCanvas => {
                println!("{} {}", self.raw_point_count, self.processed_point_count);
                self.raw_point_count = 0;
                self.processed_point_count = 0;
                if let Some(mut queue) = self.write::<BrushPointQueue>() {
                    queue.clear();
                }
                if let (Some(canvas_ctx), Some(render_ctx)) = (
                    &mut self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                ) {
                    canvas_ctx.clear_render_texture(&render_ctx);
                }
            }
            // TODO: cleanup the transformation code
            CustomEvent::CameraMove { position } => {
                if let (Some(window_res), Some(canvas_ctx), Some(render_ctx), Some(mut state)) = (
                    self.read::<WindowResource>(),
                    &mut self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                    self.write::<State>(),
                ) {
                    let window_size = window_res.0.inner_size();
                    let world_translation = screen_to_world_position(
                        position,
                        #[allow(clippy::cast_precision_loss)]
                        (window_size.width as f32, window_size.height as f32),
                    );
                    let transform = CameraTransform {
                        translation: Some(clamp::clamp_point(world_translation)),
                        ..Default::default()
                    };
                    state.camera.update(&transform);
                    canvas_ctx.update_camera_buffer(&render_ctx, &state.camera);
                }
            }
            // TODO: cleanup the transformation code
            CustomEvent::CameraZoom { delta } => {
                if let (Some(canvas_ctx), Some(render_ctx), Some(mut state)) = (
                    &mut self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                    self.write::<State>(),
                ) {
                    let transform = CameraTransform {
                        scale_delta: Some(delta),
                        ..Default::default()
                    };
                    state.camera.update(&transform);
                    canvas_ctx.update_camera_buffer(&render_ctx, &state.camera);
                }
            }
            // TODO: cleanup the transformation code
            CustomEvent::BrushPoint { dot } => {
                println!("{} {}", dot.position.x, dot.position.y);
                self.processed_point_count += 1;

                if let (Some(window), Some(state), Some(mut queue)) = (
                    self.read::<WindowResource>(),
                    self.read::<State>(),
                    self.write::<BrushPointQueue>(),
                ) {
                    let window_size = window.0.inner_size();

                    let brush_position = world_to_ndc(
                        dot.position,
                        #[allow(clippy::cast_precision_loss)]
                        (window_size.width as f32, window_size.height as f32),
                    );
                    let clamped_position = clamp::clamp_point(brush_position);

                    queue.enqueue(
                        Dot2D {
                            position: clamped_position,
                            radius: dot.radius,
                        },
                        state.camera.clone(),
                    );
                }
            }
            CustomEvent::UpdateBrushColor(color) => {
                if let (Some(mut state), Some(mut canvas_ctx), Some(render_ctx)) = (
                    self.write::<State>(),
                    self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                ) {
                    state.editor.update_brush_color(color);
                    canvas_ctx.update_brush_color(&render_ctx, color.to_rgba_array());
                }
            }
            // this is useful for syncing UI with tools eg. UI needs to show a larger brush pointer when zoomed in
            CustomEvent::_UiUpdate => {}
            CustomEvent::CanvasCreated { state } => {
                self.insert_resource(*state);
            }
        }

        if let Some(window) = &self.read::<WindowResource>() {
            window.0.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(mut render_ctx) = self.write::<RenderContext>() {
                    render_ctx.reconfigure(size);
                }
            }
            WindowEvent::RedrawRequested => {
                self.run_update_systems();
                // Cap framerate
                #[cfg(not(target_arch = "wasm32"))]
                sleep(Duration::from_millis(5));
                if let Some(window_res) = self.read::<WindowResource>() {
                    window_res.0.request_redraw();
                }
            }
            event => {
                self.raw_point_count += 1;

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

                if let Some(mut input_system) = self.write::<InputSystem>() {
                    input_system.process_event(&event);
                }

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
