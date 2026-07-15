use crate::document::{Document, LayerId};
use crate::input::selection::SelectionStack;
use crate::resource::Resource;

/// CPU side of the document (multi-artboard.md §2.3). Event handlers mutate
/// `document` and push `GpuOp`s; `PaintSystem` drains `gpu_dirty` at the top
/// of its run — the winit-event path never touches wgpu.
pub struct DocumentState {
    pub document: Document,
    pub selection: SelectionStack,
    pub gpu_dirty: Vec<GpuOp>,
}

impl DocumentState {
    /// The first artboard is selected at boot (auto-selecting its topmost
    /// layer, §3.1), so drawing works immediately.
    pub fn new(document: Document) -> Self {
        let mut selection = SelectionStack::new();
        if let Some(artboard) = document.artboards.first() {
            selection.select_artboard(&document, artboard.id);
        }
        Self {
            document,
            selection,
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
