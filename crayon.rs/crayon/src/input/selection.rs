use crate::document::{ArtboardId, LayerId};

pub enum SelectionCtx {
    Global,
    Artboard(ArtboardId),
    Layer(ArtboardId, LayerId),
}

pub struct SelectionStack {
    stack: Vec<SelectionCtx>,
}

impl Default for SelectionStack {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStack {
    pub fn new() -> Self {
        Self {
            stack: vec![SelectionCtx::Global],
        }
    }
}
