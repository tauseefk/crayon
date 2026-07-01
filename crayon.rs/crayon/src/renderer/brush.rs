pub const POINTER_SIZE: f32 = 20.0;

#[cfg(target_arch = "wasm32")]
pub const POINTER_TO_BRUSH_SIZE_MULTIPLE: f32 = 0.003;
#[cfg(not(target_arch = "wasm32"))]
pub const POINTER_TO_BRUSH_SIZE_MULTIPLE: f32 = 0.0025;

pub const DEFAULT_BRUSH_SIZE: f32 = POINTER_SIZE * POINTER_TO_BRUSH_SIZE_MULTIPLE;
