//! Test-only harness (revision T of multi-artboard-implementation.md):
//! headless GPU, texture readback, in-code document fixtures, pixel probes,
//! and `ControllerEvent` capture. Nothing here ships in the app binary — the
//! module is gated behind `#[cfg(test)]` in `lib.rs`.

#[cfg(not(target_arch = "wasm32"))]
pub mod events;
pub mod fixtures;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
pub mod probe;
