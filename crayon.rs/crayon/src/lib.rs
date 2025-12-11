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

// TODO: get rid of the prelude
mod prelude {
    pub use std::mem;
    pub use std::sync::Arc;

    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen::prelude::*;

    pub use batteries::prelude::*;
    pub use cgmath::{EuclideanSpace, Point2};

    pub use wgpu::util::DeviceExt;
    pub use winit::event_loop::EventLoopProxy;
    pub use winit::{
        application::ApplicationHandler,
        dpi::{LogicalSize, PhysicalPosition},
        event::*,
        event_loop::{ActiveEventLoop, EventLoop},
        keyboard::{KeyCode, PhysicalKey},
        window::Window,
    };

    pub use crate::app::*;
    pub use crate::brush_controller::*;
    pub use crate::camera_controller::*;
    pub use crate::constants::*;
    pub use crate::editor_state::*;
    pub use crate::event_sender::*;
    pub use crate::events::*;
    pub use crate::renderer::{brush::*, camera::*, pipeline::*};
    pub use crate::resource::*;
    pub use crate::state::*;
    pub use crate::system::*;
    pub use crate::utils::*;
}

use crate::renderer::ui::hello_widget::HelloResource;
use crate::resources::brush_point_queue::BrushPointQueue;
use crate::resources::frame_time::FrameTime;
use crate::systems::canvas_render_system::CanvasRenderSystem;
use crate::systems::frame_acquire_system::FrameAcquireSystem;
use crate::systems::frame_present_system::FramePresentSystem;
use crate::systems::frame_time_update::FrameTimeUpdateSystem;
use crate::systems::paint_system::PaintSystem;
use crate::systems::tools_system::ToolsSystem;
use prelude::*;

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

    app.insert_resource(FrameTime::new());
    app.insert_resource(BrushPointQueue::new());
    app.insert_resource(HelloResource::new());

    app.add_system(Schedule::PreUpdate, FrameAcquireSystem)
        .add_system(Schedule::Update, FrameTimeUpdateSystem)
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
