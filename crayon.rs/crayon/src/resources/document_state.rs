use crate::document::{Document, LayerId};
use crate::resource::Resource;

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

pub enum GpuOp {
    ClearLayer { layer: LayerId },
}

impl Resource for DocumentState {}
