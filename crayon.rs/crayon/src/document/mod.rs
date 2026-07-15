//! The document model (§1 of multi-artboard.md): artboards, layers, and their
//! raster content references. Plain data — knows nothing about wgpu. GPU-side
//! objects are hydrated from this model at load time.

pub mod loader;
pub mod thumbhash;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(transparent)]
pub struct ArtboardId(pub u32);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(transparent)]
pub struct LayerId(pub u32);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Document {
    pub version: u32,
    /// Monotonic id allocator; artboards and layers share it.
    pub next_id: u32,
    /// Draw order = index order (later = on top).
    pub artboards: Vec<Artboard>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Artboard {
    pub id: ArtboardId,
    pub name: String,
    /// Top-left corner, world px.
    pub position: [f32; 2],
    /// World px; clamped to the device max texture dimension on load.
    pub size: [f32; 2],
    /// Bottom-to-top.
    pub layers: Vec<Layer>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    /// Artboard-local px; repositioning mutates only this.
    pub offset: [f32; 2],
    pub visible: bool,
    /// Relative path of a PNG next to the JSON file; None = blank (transparent) layer.
    pub content: Option<String>,
    /// Base64 thumbhash of the layer content for instant previews. None for blank layers.
    pub thumbhash: Option<String>,
}

pub const DOCUMENT_VERSION: u32 = 1;

/// World px. Under the 2048 WebGL texture ceiling, so `add_artboard` needs no
/// per-device clamp (only loaded documents are clamped, §1.5).
pub const DEFAULT_ARTBOARD_SIZE: [f32; 2] = [800.0, 600.0];

/// Horizontal gap between the existing document and a newly added artboard.
const NEW_ARTBOARD_GAP: f32 = 40.0;

impl Document {
    /// 1 artboard "Artboard 1" 800x600 @ (0,0) with 1 blank layer — the
    /// fallback whenever loading a document from assets fails.
    pub fn default_document() -> Self {
        let mut document = Self {
            version: DOCUMENT_VERSION,
            next_id: 1,
            artboards: Vec::new(),
        };
        let artboard_id = document.alloc_artboard_id();
        let layer_id = document.alloc_layer_id();
        document.artboards.push(Artboard {
            id: artboard_id,
            name: "Artboard 1".to_string(),
            position: [0.0, 0.0],
            size: [800.0, 600.0],
            layers: vec![Layer {
                id: layer_id,
                name: "Layer 1".to_string(),
                offset: [0.0, 0.0],
                visible: true,
                content: None,
                thumbhash: None,
            }],
        });
        document
    }

    pub fn alloc_artboard_id(&mut self) -> ArtboardId {
        let id = self.next_id;
        self.next_id += 1;
        ArtboardId(id)
    }

    pub fn alloc_layer_id(&mut self) -> LayerId {
        let id = self.next_id;
        self.next_id += 1;
        LayerId(id)
    }

    pub fn artboard(&self, id: ArtboardId) -> Option<&Artboard> {
        self.artboards.iter().find(|artboard| artboard.id == id)
    }

    pub fn artboard_mut(&mut self, id: ArtboardId) -> Option<&mut Artboard> {
        self.artboards.iter_mut().find(|artboard| artboard.id == id)
    }

    pub fn find_layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.artboards
            .iter_mut()
            .find_map(|artboard| artboard.layers.iter_mut().find(|layer| layer.id == id))
    }

    pub fn find_layer(&self, id: LayerId) -> Option<(ArtboardId, &Layer)> {
        self.artboards.iter().find_map(|artboard| {
            artboard
                .layers
                .iter()
                .find(|layer| layer.id == id)
                .map(|layer| (artboard.id, layer))
        })
    }

    /// Appends a default-sized artboard with one blank layer, placed to the
    /// right of the document's bounding box (at the origin when empty).
    pub fn add_artboard(&mut self) -> ArtboardId {
        let position = self.next_artboard_position();
        let artboard_id = self.alloc_artboard_id();
        let layer_id = self.alloc_layer_id();
        self.artboards.push(Artboard {
            id: artboard_id,
            name: format!("Artboard {}", artboard_id.0),
            position,
            size: DEFAULT_ARTBOARD_SIZE,
            layers: vec![Layer::blank(layer_id)],
        });
        artboard_id
    }

    /// Removes the artboard, returning it (with its layers) so the caller can
    /// release the matching GPU resources. `None` when the id is unknown.
    pub fn remove_artboard(&mut self, id: ArtboardId) -> Option<Artboard> {
        let index = self
            .artboards
            .iter()
            .position(|artboard| artboard.id == id)?;
        Some(self.artboards.remove(index))
    }

    /// Pushes a blank layer on top of the artboard's stack. `None` when the
    /// artboard id is unknown.
    pub fn add_layer(&mut self, artboard: ArtboardId) -> Option<LayerId> {
        let index = self
            .artboards
            .iter()
            .position(|candidate| candidate.id == artboard)?;
        let layer_id = self.alloc_layer_id();
        self.artboards[index].layers.push(Layer::blank(layer_id));
        Some(layer_id)
    }

    /// Removes the layer from its owning artboard, returning both so the
    /// caller can release the matching GPU resources.
    pub fn remove_layer(&mut self, id: LayerId) -> Option<(ArtboardId, Layer)> {
        for artboard in &mut self.artboards {
            if let Some(index) = artboard.layers.iter().position(|layer| layer.id == id) {
                return Some((artboard.id, artboard.layers.remove(index)));
            }
        }
        None
    }

    fn next_artboard_position(&self) -> [f32; 2] {
        if self.artboards.is_empty() {
            return [0.0, 0.0];
        }
        let right = self
            .artboards
            .iter()
            .map(|artboard| artboard.position[0] + artboard.size[0])
            .fold(f32::MIN, f32::max);
        let top = self
            .artboards
            .iter()
            .map(|artboard| artboard.position[1])
            .fold(f32::MAX, f32::min);
        [right + NEW_ARTBOARD_GAP, top]
    }

    /// Topmost artboard whose world rect contains `world`: artboards are
    /// iterated in reverse draw order, point-in-rect on (position, size).
    pub fn hit_test(&self, world_position: cgmath::Point2<f32>) -> Option<ArtboardId> {
        self.artboards
            .iter()
            .rev()
            .find(|artboard| artboard.contains(world_position))
            .map(|artboard| artboard.id)
    }
}

impl Layer {
    fn blank(id: LayerId) -> Self {
        Self {
            id,
            name: format!("Layer {}", id.0),
            offset: [0.0, 0.0],
            visible: true,
            content: None,
            thumbhash: None,
        }
    }
}

impl Artboard {
    pub fn contains(&self, world_position: cgmath::Point2<f32>) -> bool {
        world_position.x >= self.position[0]
            && world_position.x < self.position[0] + self.size[0]
            && world_position.y >= self.position[1]
            && world_position.y < self.position[1] + self.size[1]
    }

    /// Texture dimensions for this artboard's layers (layer rasters are
    /// exactly artboard-sized; `Layer::offset` is a pure transform).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn pixel_size(&self) -> (u32, u32) {
        (
            (self.size[0].round() as u32).max(1),
            (self.size[1].round() as u32).max(1),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Point2;

    fn artboard(id: u32, position: [f32; 2], size: [f32; 2]) -> Artboard {
        Artboard {
            id: ArtboardId(id),
            name: format!("Artboard {id}"),
            position,
            size,
            layers: Vec::new(),
        }
    }

    #[test]
    fn serde_round_trip() {
        let document = Document::default_document();
        let json = serde_json::to_string(&document).unwrap();
        let parsed: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(document, parsed);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn default_document_shape() {
        let document = Document::default_document();
        assert_eq!(document.version, DOCUMENT_VERSION);
        assert_eq!(document.artboards.len(), 1);
        let artboard = &document.artboards[0];
        assert_eq!(artboard.name, "Artboard 1");
        assert_eq!(artboard.position, [0.0, 0.0]);
        assert_eq!(artboard.size, [800.0, 600.0]);
        assert_eq!(artboard.layers.len(), 1);
        let layer = &artboard.layers[0];
        assert!(layer.visible);
        assert!(layer.content.is_none());
        assert!(layer.thumbhash.is_none());
        assert_eq!(layer.offset, [0.0, 0.0]);
    }

    #[test]
    fn id_allocation_is_monotonic_and_shared() {
        let mut document = Document::default_document();
        let next = document.next_id;
        let a = document.alloc_artboard_id();
        let l = document.alloc_layer_id();
        let b = document.alloc_artboard_id();
        assert_eq!(a.0, next);
        assert_eq!(l.0, next + 1);
        assert_eq!(b.0, next + 2);
        assert_eq!(document.next_id, next + 3);
    }

    #[test]
    fn hit_test_topmost_wins_on_overlap() {
        let document = Document {
            version: DOCUMENT_VERSION,
            next_id: 3,
            artboards: vec![
                artboard(1, [0.0, 0.0], [200.0, 200.0]),
                artboard(2, [100.0, 100.0], [200.0, 200.0]),
            ],
        };
        // Overlap region: the later artboard (index 1) is drawn on top.
        assert_eq!(
            document.hit_test(Point2::new(150.0, 150.0)),
            Some(ArtboardId(2))
        );
        // Only inside the first artboard.
        assert_eq!(
            document.hit_test(Point2::new(50.0, 50.0)),
            Some(ArtboardId(1))
        );
        // Only inside the second artboard.
        assert_eq!(
            document.hit_test(Point2::new(250.0, 250.0)),
            Some(ArtboardId(2))
        );
    }

    #[test]
    fn hit_test_miss_is_none() {
        let document = Document {
            version: DOCUMENT_VERSION,
            next_id: 2,
            artboards: vec![artboard(1, [0.0, 0.0], [200.0, 200.0])],
        };
        assert_eq!(document.hit_test(Point2::new(-1.0, 50.0)), None);
        assert_eq!(document.hit_test(Point2::new(200.0, 200.0)), None);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn add_artboard_places_right_of_bounding_box() {
        let mut document = Document::default_document();
        let id = document.add_artboard();
        let artboard = document.artboard(id).unwrap();
        // Existing artboard spans x 0..800 at y 0.
        assert_eq!(artboard.position, [800.0 + NEW_ARTBOARD_GAP, 0.0]);
        assert_eq!(artboard.size, DEFAULT_ARTBOARD_SIZE);
        assert_eq!(artboard.layers.len(), 1, "one blank layer");
        assert!(artboard.layers[0].content.is_none());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn add_artboard_to_empty_document_starts_at_origin() {
        let mut document = Document {
            version: DOCUMENT_VERSION,
            next_id: 1,
            artboards: Vec::new(),
        };
        let id = document.add_artboard();
        assert_eq!(document.artboard(id).unwrap().position, [0.0, 0.0]);
    }

    #[test]
    fn add_layer_pushes_on_top() {
        let mut document = Document::default_document();
        let artboard_id = document.artboards[0].id;
        let bottom = document.artboards[0].layers[0].id;
        let top = document.add_layer(artboard_id).unwrap();
        let layers = &document.artboards[0].layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].id, bottom);
        assert_eq!(layers[1].id, top, "new layer is topmost");
        assert!(document.add_layer(ArtboardId(999)).is_none());
    }

    #[test]
    fn remove_layer_returns_owner_and_layer() {
        let mut document = Document::default_document();
        let layer_id = document.artboards[0].layers[0].id;
        let (owner, layer) = document.remove_layer(layer_id).unwrap();
        assert_eq!(owner, document.artboards[0].id);
        assert_eq!(layer.id, layer_id);
        assert!(document.artboards[0].layers.is_empty());
        assert!(document.remove_layer(layer_id).is_none(), "already removed");
    }

    #[test]
    fn artboards_and_layers_go_to_zero_and_back() {
        let mut document = Document::default_document();
        let id = document.artboards[0].id;
        assert!(document.remove_artboard(id).is_some());
        assert!(document.artboards.is_empty(), "zero artboards is legal");
        assert!(document.remove_artboard(id).is_none());

        // Ids never repeat across the delete/create cycle: the default
        // document consumed 1 and 2, the new artboard takes 3 (+ blank layer
        // 4) and the added layer 5.
        let id = document.add_artboard();
        let layer = document.add_layer(id).unwrap();
        assert_eq!(id, ArtboardId(3));
        assert_eq!(layer, LayerId(5));
        assert!(document.find_layer(layer).is_some());
    }

    #[test]
    fn find_layer_reports_owning_artboard() {
        let document = Document::default_document();
        let artboard = &document.artboards[0];
        let layer_id = artboard.layers[0].id;
        let (owner, layer) = document.find_layer(layer_id).unwrap();
        assert_eq!(owner, artboard.id);
        assert_eq!(layer.id, layer_id);
        assert!(document.find_layer(LayerId(9999)).is_none());
    }
}
