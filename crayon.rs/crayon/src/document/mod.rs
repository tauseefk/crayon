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
    /// Monotonic id allocator, shared between artboards and layers
    pub next_id: u32,
    /// Drawn in ascending order of index
    pub artboards: Vec<Artboard>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Artboard {
    pub id: ArtboardId,
    pub name: String,
    /// Top-left corner in world position
    pub position: [f32; 2],
    /// Size in world coordinates clamped to device max texture dims on load.
    pub size: [f32; 2],
    /// Drawn in ascending order of index
    pub layers: Vec<Layer>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    /// Artboard local top left corner.
    pub offset: [f32; 2],
    pub visible: bool,
    /// Relative path for bundled assets, or web url
    /// None signifies an empty layer.
    pub content_path: Option<String>,
    /// Base64 thumbhash of the layer content for instant previews.
    /// None for empty layer.
    pub thumbhash: Option<String>,
}

pub const DOCUMENT_VERSION: u32 = 1;

impl Default for Document {
    fn default() -> Self {
        let mut doc = Self {
            version: DOCUMENT_VERSION,
            next_id: 1,
            artboards: vec![],
        };

        let artboard_id = doc.alloc_artboard_id();
        let layer_id = doc.alloc_layer_id();

        doc.artboards.push(Artboard {
            id: artboard_id,
            name: "Artboard 1".to_string(),
            position: [0.0, 0.0],
            size: [800.0, 600.0],
            layers: vec![],
        });
        doc
    }
}

impl Document {
    pub fn default_document() -> Self {
        Self::default()
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

    pub fn find_layer(&self, id: LayerId) -> Option<(ArtboardId, &Layer)> {
        self.artboards.iter().find_map(|artboard| {
            artboard
                .layers
                .iter()
                .find(|layer| layer.id == id)
                .map(|layer| (artboard.id, layer))
        })
    }

    pub fn hit_test(&self, world_position: cgmath::Point2<f32>) -> Option<ArtboardId> {
        self.artboards
            .iter()
            .rev()
            .find(|artboard| artboard.contains(world_position))
            .map(|artboard| artboard.id)
    }
}

impl Artboard {
    pub fn contains(&self, world_position: cgmath::Point2<f32>) -> bool {
        world_position.x >= self.position[0]
            && world_position.x < self.position[0] + self.size[0]
            && world_position.y >= self.position[1]
            && world_position.y < self.position[1] + self.size[1]
    }

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
        assert!(layer.content_path.is_none());
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
