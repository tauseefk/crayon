use crate::document::{Document, LayerId};
use crate::resource::Resource;

/// CPU side of the document (multi-artboard.md §2.3). Event handlers mutate
/// `document` and push `GpuOp`s; `PaintSystem` drains `gpu_dirty` at the top
/// of its run — the winit-event path never touches wgpu.
pub struct DocumentState {
    pub document: Document,
    pub gpu_dirty: Vec<GpuOp>,
}

impl DocumentState {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            gpu_dirty: Vec::new(),
        }
    }
}

/// Structural changes to `SceneRenderer` textures. Create/destroy variants
/// arrive with the selection and panel stages (S4/S5).
pub enum GpuOp {
    ClearLayer { layer: LayerId },
}

impl Resource for DocumentState {}
