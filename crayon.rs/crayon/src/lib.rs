#![warn(clippy::pedantic)]

mod app;
mod brush_controller;
mod camera_controller;
mod constants;
mod editor_state;
mod event_sender;
mod events;
mod renderer;
mod state;
mod texture;
mod utils;

mod prelude {
    pub use std::mem;
    pub use std::sync::Arc;

    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen::prelude::*;

    pub use batteries::prelude::*;
    pub use cgmath::{EuclideanSpace, Point2};

    pub use wgpu::MemoryHints;
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
    pub use crate::renderer::{brush::*, camera::*, pipeline::*, state::*};
    pub use crate::state::*;
    pub use crate::texture::*;
    pub use crate::utils::*;
}

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
