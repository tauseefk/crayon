//! Hotkey / selection re-architecture (multi-artboard.md §3): the selection
//! stack, the normalized `InputAction` dispatch, and the per-context handlers.

pub mod artboard_handler;
pub mod dispatch;
pub mod global_handler;
pub mod layer_handler;
pub mod selection;
