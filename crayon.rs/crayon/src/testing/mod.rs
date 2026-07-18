//! Test-only harness with:
//! - headless GPU
//! - texture readback
//! - test document fixtures
//! - pixel probes
//! - `ControllerEvent` capture

#[cfg(not(target_arch = "wasm32"))]
pub mod events;
pub mod fixtures;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
pub mod probe;
