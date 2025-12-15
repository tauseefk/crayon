use crate::{
    prelude::*,
    renderer::{
        egui_context::EguiContext, frame_context::FrameContext, render_context::RenderContext,
    },
    resources::{
        brush_point_queue::BrushPointQueue, brush_preview_state::BrushPreviewState,
        canvas_state::CanvasContext, input_system::InputSystem,
    },
};

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
    #[cfg(target_arch = "wasm32")]
    event_loop_proxy: EventLoopProxy<CustomEvent>,
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
            #[cfg(target_arch = "wasm32")]
            event_loop_proxy: event_loop_proxy.clone(),
        };

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
        for system in &self.pre_update_systems {
            system.run(self);
        }

        for system in &self.update_systems {
            system.run(self);
        }

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
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.read::<WindowResource>() {
            window.0.request_redraw();
        }
    }
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

            #[cfg(target_arch = "wasm32")]
            {
                let proxy = self.event_loop_proxy.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let render_context = RenderContext::new(window.clone())
                        .await
                        .expect("Unable to create canvas!!!");
                    let _ = proxy.send_event(CustomEvent::CanvasCreated {
                        render_context: Box::new(render_context),
                        window: window.clone(),
                    });
                });
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let render_context = pollster::block_on(RenderContext::new(window.clone()))
                    .expect("Unable to create canvas!!!");
                let window_size = window.inner_size();
                let canvas_state =
                    CanvasContext::new(&render_context, (window_size.width, window_size.height));
                let egui_context = EguiContext::new(window.clone(), &render_context);
                let app_state = State::new(window_size.width, window_size.height);

                self.insert_resource(render_context)
                    .insert_resource(canvas_state)
                    .insert_resource(egui_context)
                    .insert_resource(app_state)
                    .insert_resource(FrameContext::new())
                    .insert_resource(WindowResource(window));

                self.run_update_systems();
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::ClearCanvas => {
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
                if let (
                    Some(canvas_ctx),
                    Some(render_ctx),
                    Some(mut state),
                    Some(mut preview_state),
                ) = (
                    &mut self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                    self.write::<State>(),
                    self.write::<BrushPreviewState>(),
                ) {
                    let transform = CameraTransform {
                        scale_delta: Some(delta),
                        ..Default::default()
                    };
                    state.camera.update(&transform);
                    canvas_ctx.update_camera_buffer(&render_ctx, &state.camera);
                    // Update brush preview scale to match user zoom
                    preview_state.update_scale(delta);
                }
            }
            // TODO: cleanup the transformation code
            CustomEvent::BrushPoint { dot } => {
                if let (Some(window), Some(state), Some(mut queue)) = (
                    self.read::<WindowResource>(),
                    self.read::<State>(),
                    self.write::<BrushPointQueue>(),
                ) {
                    let window_size = window.0.inner_size();

                    let position = screen_to_ndc(
                        dot.position,
                        #[allow(clippy::cast_precision_loss)]
                        (window_size.width as f32, window_size.height as f32),
                    );

                    queue.write(
                        Dot2D {
                            position,
                            radius: dot.radius,
                        },
                        state.camera.clone(),
                    );
                }
            }
            CustomEvent::UpdateBrush(properties) => {
                if let (Some(mut state), Some(mut canvas_ctx), Some(render_ctx)) = (
                    self.write::<State>(),
                    self.write::<CanvasContext>(),
                    self.read::<RenderContext>(),
                ) {
                    state.editor.update_brush(properties);
                    canvas_ctx.update_brush(
                        &render_ctx,
                        properties.color.to_rgba_array(),
                        properties.size,
                    );
                }
            }
            CustomEvent::_UiUpdate => {}
            CustomEvent::CanvasCreated {
                render_context,
                window,
            } => {
                #[allow(unused_mut)]
                let mut window_size = window.inner_size();
                // Use the same size override as RenderContext
                #[cfg(target_arch = "wasm32")]
                {
                    window_size.width = WINDOW_SIZE.0;
                    window_size.height = WINDOW_SIZE.1;
                }

                let canvas_ctx =
                    CanvasContext::new(&render_context, (window_size.width, window_size.height));
                let egui_ctx = EguiContext::new(window.clone(), &render_context);
                let app_state = State::new(window_size.width, window_size.height);

                self.insert_resource(*render_context)
                    .insert_resource(canvas_ctx)
                    .insert_resource(egui_ctx)
                    .insert_resource(app_state)
                    .insert_resource(FrameContext::new())
                    .insert_resource(WindowResource(window));

                self.run_update_systems();
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
            WindowEvent::Resized(new_size) => {
                if let (Some(mut render_ctx), Some(mut canvas_ctx), Some(mut state)) = (
                    self.write::<RenderContext>(),
                    self.write::<CanvasContext>(),
                    self.write::<State>(),
                ) {
                    if new_size.width > 0 && new_size.height > 0 {
                        state
                            .camera
                            .update_viewport(new_size.width as f32, new_size.height as f32);
                        // camera buffer needs to be updated after updating the camera
                        canvas_ctx.update_camera_buffer(&render_ctx, &state.camera);
                        render_ctx.reconfigure(new_size);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.run_update_systems();
            }
            event => {
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

                if let (Some(mut input_system), Some(state)) =
                    (self.write::<InputSystem>(), self.read::<State>())
                {
                    input_system.process_event(&event, state.editor.brush_properties.size);
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
