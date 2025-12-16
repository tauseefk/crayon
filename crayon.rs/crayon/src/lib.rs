#![warn(clippy::pedantic)]

mod app;
mod brush_controller;
mod camera_controller;
mod constants;
mod editor_state;
mod event_sender;
mod events;
mod renderer;
mod resource;
mod resources;
mod state;
mod system;
mod systems;
mod texture;
mod utils;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::event_loop::EventLoop;

use crate::app::App;
use crate::renderer::ui::hello_widget::HelloResource;
use crate::resource::ResourceContext;
use crate::resources::brush_point_queue::BrushPointQueue;
use crate::resources::brush_preview_state::BrushPreviewState;
use crate::resources::frame_time::FrameTime;
use crate::system::{Schedule, SystemRegistry};
use crate::systems::brush_preview_update_system::BrushPreviewUpdateSystem;
use crate::systems::canvas_render_system::CanvasRenderSystem;
use crate::systems::frame_acquire_system::FrameAcquireSystem;
use crate::systems::frame_present_system::FramePresentSystem;
use crate::systems::frame_time_update::FrameTimeUpdateSystem;
use crate::systems::paint_system::PaintSystem;
use crate::systems::tools_system::ToolsSystem;

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let event_loop_proxy = event_loop.create_proxy();
    let mut app = App::new(event_loop_proxy);

    app.insert_resource(FrameTime::new())
        .insert_resource(BrushPointQueue::new())
        .insert_resource(HelloResource::new())
        .insert_resource(BrushPreviewState::new());

    app.add_system(Schedule::PreUpdate, FrameAcquireSystem)
        .add_system(Schedule::Update, FrameTimeUpdateSystem)
        .add_system(Schedule::Update, BrushPreviewUpdateSystem)
        .add_system(Schedule::Update, PaintSystem)
        .add_system(Schedule::Update, CanvasRenderSystem)
        .add_system(Schedule::Update, ToolsSystem::new())
        .add_system(Schedule::PostUpdate, FramePresentSystem);

    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
