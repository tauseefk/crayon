use cgmath::{EuclideanSpace, Point2};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::document::loader::load_document;
#[cfg(not(target_arch = "wasm32"))]
use crate::resources::launch_options::LaunchOptions;
use crate::{
    constants::WINDOW_SIZE,
    document::{Document, loader::LoadedDocument},
    event_sender::EventSender,
    events::CustomEvent,
    input::dispatch::DispatchEnv,
    renderer::{
        egui_context::EguiContext, frame_context::FrameContext, render_context::RenderContext,
    },
    resource::{Res, ResMut, Resource, ResourceContext},
    resources::{
        brush_point_queue::BrushPointQueue,
        brush_preview_state::BrushPreviewState,
        document_state::{DocumentState, GpuOp},
        input_system::InputSystem,
        scene_renderer::SceneRenderer,
        stroke_state::StrokeState,
    },
    state::State,
    system::{Schedule, System, SystemRegistry},
};

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub struct WindowResource(pub Arc<winit::window::Window>);
impl Resource for WindowResource {}

/// World-px center of the document's artboard bounding box — the boot camera
/// target. Origin for documents with no artboards.
fn document_center(document: &Document) -> Point2<f32> {
    let mut min = Point2::new(f32::MAX, f32::MAX);
    let mut max = Point2::new(f32::MIN, f32::MIN);
    for artboard in &document.artboards {
        min.x = min.x.min(artboard.position[0]);
        min.y = min.y.min(artboard.position[1]);
        max.x = max.x.max(artboard.position[0] + artboard.size[0]);
        max.y = max.y.max(artboard.position[1] + artboard.size[1]);
    }
    if document.artboards.is_empty() {
        Point2::origin()
    } else {
        Point2::new(f32::midpoint(min.x, max.x), f32::midpoint(min.y, max.y))
    }
}

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

        app.insert_resource(event_sender);
        app.insert_resource(InputSystem::new());

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

    /// Builds the document-backed scene: `SceneRenderer` hydrated from
    /// `loaded`, the `DocumentState` resource, and app `State` with the
    /// camera centered on the document.
    fn insert_document_resources(
        &mut self,
        render_context: &RenderContext,
        loaded: LoadedDocument,
        window_size: (u32, u32),
    ) {
        let mut scene_renderer = SceneRenderer::new(
            &render_context.device,
            &render_context.queue,
            render_context.config.format,
        );
        scene_renderer.hydrate(&render_context.device, &render_context.queue, &loaded);

        let mut app_state = State::new(window_size.0, window_size.1);
        app_state.camera.center_on(document_center(&loaded.document));

        self.insert_resource(scene_renderer)
            .insert_resource(DocumentState::new(loaded.document))
            .insert_resource(app_state)
            .insert_resource(FrameContext::new());
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

    /// Resumed
    ///
    /// This handles two critical things:
    /// - window creation
    /// - resource insertion into `App`
    ///
    /// On WASM target, window is first created with zero size,
    /// so the actual resource creation (renderer, canvas, etc) is done via `CustomEvent::CanvasCreated`
    ///
    /// Resources are inserted here for Non-WASM targets.
    ///
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

            // WindowResource can be inserted for all platforms
            self.insert_resource(WindowResource(window.clone()));

            #[cfg(not(target_arch = "wasm32"))]
            {
                let window_size = window.inner_size();
                let render_context = pollster::block_on(RenderContext::new(window.clone()))
                    .expect("Unable to create canvas!!!");

                let document_name = self
                    .read::<LaunchOptions>()
                    .map_or_else(|| "default".to_string(), |options| options.document.clone());
                let max_texture_dim = render_context.device.limits().max_texture_dimension_2d;
                let loaded =
                    load_document(&document_name, max_texture_dim).unwrap_or_else(|error| {
                        log::warn!(
                            "failed to load document '{document_name}': {error:#}; \
                             falling back to the default document"
                        );
                        LoadedDocument {
                            document: Document::default_document(),
                            layer_pixels: HashMap::new(),
                        }
                    });

                let egui_context = EguiContext::new(window, &render_context);

                self.insert_document_resources(
                    &render_context,
                    loaded,
                    (window_size.width, window_size.height),
                );
                self.insert_resource(render_context)
                    .insert_resource(egui_context);

                self.run_update_systems();
            }
        }
    }

    /// User Event Handler
    ///
    /// This handles all `CustomEvent` instances.
    /// For the WASM target, it handles (renderer, canvas, state, etc) resource creation and insertion as well.
    ///
    // one match arm per event variant; splitting would only scatter the dispatch
    #[allow(clippy::too_many_lines)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        match event {
            CustomEvent::CameraMove { world_delta } => {
                if let Some(mut state) = self.write::<State>() {
                    state.camera.pan_world_delta(world_delta);
                }
            }
            CustomEvent::SelectArtboard(artboard) => {
                if let Some(mut doc) = self.write::<DocumentState>() {
                    let DocumentState {
                        document,
                        selection,
                        ..
                    } = &mut *doc;
                    selection.select_artboard(document, artboard);
                }
            }
            CustomEvent::SelectLayer(artboard, layer) => {
                if let Some(mut doc) = self.write::<DocumentState>() {
                    doc.selection.select_layer(artboard, layer);
                }
            }
            CustomEvent::ClearSelection => {
                if let Some(mut doc) = self.write::<DocumentState>() {
                    doc.selection.clear();
                }
            }
            // Move* are pure-CPU offset/position mutations: the quad origin
            // changes next frame, zero GPU work (§3.4).
            CustomEvent::MoveLayer { layer, world_delta } => {
                if let Some(mut doc) = self.write::<DocumentState>()
                    && let Some(layer) = doc.document.find_layer_mut(layer)
                {
                    layer.offset[0] += world_delta.x;
                    layer.offset[1] += world_delta.y;
                }
            }
            CustomEvent::MoveArtboard {
                artboard,
                world_delta,
            } => {
                if let Some(mut doc) = self.write::<DocumentState>()
                    && let Some(artboard) = doc.document.artboard_mut(artboard)
                {
                    artboard.position[0] += world_delta.x;
                    artboard.position[1] += world_delta.y;
                }
            }
            // Handlers only push GpuOps — PaintSystem applies them.
            CustomEvent::ClearLayer { layer } => {
                if let Some(mut doc) = self.write::<DocumentState>() {
                    doc.gpu_dirty.push(GpuOp::ClearLayer { layer });
                }
            }
            CustomEvent::CameraZoom { delta } => {
                if let (Some(mut state), Some(mut preview_state)) =
                    (self.write::<State>(), self.write::<BrushPreviewState>())
                {
                    state.camera.zoom_by(delta);
                    // Update brush preview scale to match viewport zoom
                    preview_state.update_scale(delta);
                }
            }
            CustomEvent::BrushPoint { dot } => {
                if let (Some(state), Some(stroke_state), Some(mut queue)) = (
                    self.read::<State>(),
                    self.read::<StrokeState>(),
                    self.write::<BrushPointQueue>(),
                ) {
                    // Raw screen px + camera snapshot + target captured at
                    // enqueue time; PaintSystem does the transform chain.
                    queue.write(dot, state.camera, stroke_state.target);
                }
            }
            CustomEvent::UpdateBrush(properties) => {
                if let (Some(mut state), Some(mut scene), Some(render_ctx)) = (
                    self.write::<State>(),
                    self.write::<SceneRenderer>(),
                    self.read::<RenderContext>(),
                ) {
                    state.editor.update_brush(properties);
                    scene.update_brush(&render_ctx.queue, properties.color.to_rgba_array());
                }
            }
            CustomEvent::StrokeStart => {
                if let (Some(doc), Some(mut stroke_state)) =
                    (self.read::<DocumentState>(), self.write::<StrokeState>())
                {
                    // The stroke targets the selected layer; dropped when
                    // there is none (§3.4).
                    if let Some(target) = doc.selection.selected_layer() {
                        stroke_state.start(target);
                    }
                }
            }
            CustomEvent::StrokeEnd => {
                if let Some(mut stroke_state) = self.write::<StrokeState>() {
                    stroke_state.end();
                }
            }
            // Only used by WASM target
            CustomEvent::CanvasCreated {
                render_context,
                window,
            } => {
                let window_size = window.inner_size();
                let egui_ctx = EguiContext::new(window.clone(), &render_context);

                // The async asset fetch lands in S6; until then wasm boots on
                // the default document.
                let loaded = LoadedDocument {
                    document: Document::default_document(),
                    layer_pixels: HashMap::new(),
                };
                self.insert_document_resources(
                    &render_context,
                    loaded,
                    (window_size.width, window_size.height),
                );
                self.insert_resource(*render_context)
                    .insert_resource(egui_ctx)
                    .insert_resource(WindowResource(window));

                self.run_update_systems();
            }
        }

        if let Some(window) = &self.read::<WindowResource>() {
            window.0.request_redraw();
        }
    }

    /// Window Event
    ///
    /// On WASM target, the first resize event handles creation of `RenderContext`.
    ///
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                // Ignore zero-size resize events
                if new_size.width == 0 || new_size.height == 0 {
                    return;
                }

                #[cfg(target_arch = "wasm32")]
                if self.read::<RenderContext>().is_none() {
                    let window = self.read::<WindowResource>().map(|res| res.0.clone());

                    if let Some(window) = window {
                        let proxy = self.event_loop_proxy.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            // initialize with correctly sized window
                            let render_context = RenderContext::new(window.clone())
                                .await
                                .expect("Unable to create canvas!!!");
                            let _ = proxy.send_event(CustomEvent::CanvasCreated {
                                render_context: Box::new(render_context),
                                window: window.clone(),
                            });
                        });
                    }
                    return;
                }

                // Subsequent resize handling: world px are resize-invariant,
                // only the viewport and the surface change.
                if let (Some(mut render_ctx), Some(mut state)) =
                    (self.write::<RenderContext>(), self.write::<State>())
                {
                    #[allow(clippy::cast_precision_loss)]
                    state
                        .camera
                        .update_viewport(new_size.width as f32, new_size.height as f32);
                    render_ctx.reconfigure(new_size);
                }
            }
            WindowEvent::RedrawRequested => {
                // Only run if resources are initialized
                if self.read::<RenderContext>().is_some() {
                    self.run_update_systems();
                }
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

                // Esc pops one selection frame; at [Global] already → exit
                // the app (§3.3). Handled outside the dispatcher: it needs
                // the event loop and mutable selection access.
                if let WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            state: key_state,
                            repeat: false,
                            ..
                        },
                    ..
                } = event
                    && key_state.is_pressed()
                {
                    let popped = self
                        .write::<DocumentState>()
                        .is_some_and(|mut doc| doc.selection.pop());
                    if !popped {
                        event_loop.exit();
                    }
                    return;
                }

                if let (
                    Some(mut input_system),
                    Some(state),
                    Some(doc),
                    Some(stroke_state),
                    Some(sender),
                ) = (
                    self.write::<InputSystem>(),
                    self.read::<State>(),
                    self.read::<DocumentState>(),
                    self.read::<StrokeState>(),
                    self.read::<EventSender>(),
                ) {
                    input_system.process_event(
                        &event,
                        DispatchEnv {
                            // stamped by InputSystem from its tracked state
                            modifiers: winit::keyboard::ModifiersState::default(),
                            camera: state.camera,
                            doc: &doc.document,
                            selection: &doc.selection,
                            brush_size: state.editor.brush_properties.size,
                            stroke_active: stroke_state.active_target().is_some(),
                            sender: &sender,
                        },
                    );
                }
            }
        }
    }
}
