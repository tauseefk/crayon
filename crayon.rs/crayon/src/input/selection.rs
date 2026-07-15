//! The selection stack (multi-artboard.md §3.1): nested contexts, innermost =
//! most specific — the direct analogue of the DOM event path.

use crate::document::{ArtboardId, Document, LayerId};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SelectionCtx {
    Global,
    Artboard(ArtboardId),
    Layer(ArtboardId, LayerId),
}

/// Invariant: `stack[0] == Global`. Legal states:
/// `[Global]` · `[Global, Artboard(a)]` · `[Global, Artboard(a), Layer(a, l)]`
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

    /// Dispatch order: innermost context first (bubble phase).
    pub fn contexts_inner_to_outer(&self) -> impl Iterator<Item = SelectionCtx> + '_ {
        self.stack.iter().copied().rev()
    }

    /// `[Global, Artboard(id), Layer(id, topmost)]` — the topmost layer is
    /// auto-selected; `[Global, Artboard(id)]` when the artboard has no
    /// layers. Clears to `[Global]` when `id` is not in the document.
    pub fn select_artboard(&mut self, doc: &Document, id: ArtboardId) {
        let Some(artboard) = doc.artboard(id) else {
            self.clear();
            return;
        };
        self.stack = vec![SelectionCtx::Global, SelectionCtx::Artboard(id)];
        // layers are bottom-to-top, so the topmost is the last
        if let Some(layer) = artboard.layers.last() {
            self.stack.push(SelectionCtx::Layer(id, layer.id));
        }
    }

    pub fn select_layer(&mut self, artboard: ArtboardId, layer: LayerId) {
        self.stack = vec![
            SelectionCtx::Global,
            SelectionCtx::Artboard(artboard),
            SelectionCtx::Layer(artboard, layer),
        ];
    }

    /// Esc: one frame off. Never pops `Global`; returns whether a frame was
    /// popped (false ⇒ already at `[Global]`, the caller exits the app).
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.stack.truncate(1);
    }

    pub fn selected_layer(&self) -> Option<(ArtboardId, LayerId)> {
        self.stack.iter().find_map(|ctx| match ctx {
            SelectionCtx::Layer(artboard, layer) => Some((*artboard, *layer)),
            _ => None,
        })
    }

    pub fn selected_artboard(&self) -> Option<ArtboardId> {
        self.stack.iter().find_map(|ctx| match ctx {
            SelectionCtx::Artboard(artboard) => Some(*artboard),
            _ => None,
        })
    }

    /// Pops every frame that references the deleted artboard.
    pub fn on_artboard_deleted(&mut self, id: ArtboardId) {
        self.stack.retain(|ctx| match ctx {
            SelectionCtx::Artboard(artboard) | SelectionCtx::Layer(artboard, _) => *artboard != id,
            SelectionCtx::Global => true,
        });
    }

    /// Pops the layer frame when it references the deleted layer.
    pub fn on_layer_deleted(&mut self, id: LayerId) {
        self.stack.retain(|ctx| match ctx {
            SelectionCtx::Layer(_, layer) => *layer != id,
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Artboard, DOCUMENT_VERSION, Layer};

    fn doc_two_artboards() -> Document {
        // Artboard 1 with layers 2 (bottom) and 3 (top); artboard 4 with no layers.
        Document {
            version: DOCUMENT_VERSION,
            next_id: 5,
            artboards: vec![
                Artboard {
                    id: ArtboardId(1),
                    name: "a".to_string(),
                    position: [0.0, 0.0],
                    size: [100.0, 100.0],
                    layers: vec![blank_layer(2), blank_layer(3)],
                },
                Artboard {
                    id: ArtboardId(4),
                    name: "b".to_string(),
                    position: [200.0, 0.0],
                    size: [100.0, 100.0],
                    layers: vec![],
                },
            ],
        }
    }

    fn blank_layer(id: u32) -> Layer {
        Layer {
            id: LayerId(id),
            name: format!("layer {id}"),
            offset: [0.0, 0.0],
            visible: true,
            content: None,
            thumbhash: None,
        }
    }

    fn contexts(stack: &SelectionStack) -> Vec<SelectionCtx> {
        let mut inner_to_outer: Vec<_> = stack.contexts_inner_to_outer().collect();
        inner_to_outer.reverse();
        inner_to_outer
    }

    #[test]
    fn select_artboard_auto_selects_topmost_layer() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        assert_eq!(
            contexts(&stack),
            vec![
                SelectionCtx::Global,
                SelectionCtx::Artboard(ArtboardId(1)),
                SelectionCtx::Layer(ArtboardId(1), LayerId(3)),
            ]
        );
        assert_eq!(stack.selected_layer(), Some((ArtboardId(1), LayerId(3))));
        assert_eq!(stack.selected_artboard(), Some(ArtboardId(1)));
    }

    #[test]
    fn select_artboard_without_layers_stops_at_artboard() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(4));
        assert_eq!(
            contexts(&stack),
            vec![SelectionCtx::Global, SelectionCtx::Artboard(ArtboardId(4))]
        );
        assert_eq!(stack.selected_layer(), None);
    }

    #[test]
    fn select_missing_artboard_clears() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.select_artboard(&doc, ArtboardId(99));
        assert_eq!(contexts(&stack), vec![SelectionCtx::Global]);
    }

    #[test]
    fn select_layer_replaces_the_whole_stack() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        // L → L: selecting another layer of the same artboard
        stack.select_layer(ArtboardId(1), LayerId(2));
        assert_eq!(stack.selected_layer(), Some((ArtboardId(1), LayerId(2))));
        assert_eq!(contexts(&stack).len(), 3);
    }

    #[test]
    fn pop_walks_out_one_frame_and_never_pops_global() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));

        assert!(stack.pop(), "L → A");
        assert_eq!(stack.selected_layer(), None);
        assert_eq!(stack.selected_artboard(), Some(ArtboardId(1)));

        assert!(stack.pop(), "A → G");
        assert_eq!(stack.selected_artboard(), None);

        assert!(!stack.pop(), "already at [Global]");
        assert_eq!(contexts(&stack), vec![SelectionCtx::Global]);
    }

    #[test]
    fn clear_jumps_to_global_from_any_depth() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.clear();
        assert_eq!(contexts(&stack), vec![SelectionCtx::Global]);
    }

    #[test]
    fn deleting_the_selected_layer_pops_to_artboard() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.on_layer_deleted(LayerId(3));
        assert_eq!(stack.selected_layer(), None);
        assert_eq!(stack.selected_artboard(), Some(ArtboardId(1)));
    }

    #[test]
    fn deleting_an_unrelated_layer_keeps_the_selection() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.on_layer_deleted(LayerId(2));
        assert_eq!(stack.selected_layer(), Some((ArtboardId(1), LayerId(3))));
    }

    #[test]
    fn deleting_the_selected_artboard_clears_to_global() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.on_artboard_deleted(ArtboardId(1));
        assert_eq!(contexts(&stack), vec![SelectionCtx::Global]);
    }

    #[test]
    fn deleting_an_unrelated_artboard_keeps_the_selection() {
        let doc = doc_two_artboards();
        let mut stack = SelectionStack::new();
        stack.select_artboard(&doc, ArtboardId(1));
        stack.on_artboard_deleted(ArtboardId(4));
        assert_eq!(stack.selected_layer(), Some((ArtboardId(1), LayerId(3))));
    }
}
