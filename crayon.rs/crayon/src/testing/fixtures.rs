use crate::document::{Artboard, ArtboardId, DOCUMENT_VERSION, Document, Layer, LayerId};

fn blank_layer(id: u32) -> Layer {
    Layer {
        id: LayerId(id),
        name: "Layer 1".to_string(),
        offset: [0.0, 0.0],
        visible: true,
        content_path: None,
        thumbhash: None,
    }
}

pub fn doc_single_layer() -> Document {
    Document::default_document()
}

/// Two artboards at distinct world positions, one blank layer each for placement tests.
pub fn doc_two_artboards() -> Document {
    Document {
        version: DOCUMENT_VERSION,
        next_id: 5,
        artboards: vec![
            Artboard {
                id: ArtboardId(1),
                name: "Left".to_string(),
                position: [0.0, 0.0],
                size: [600.0, 400.0],
                layers: vec![blank_layer(2)],
            },
            Artboard {
                id: ArtboardId(3),
                name: "Right".to_string(),
                position: [700.0, 100.0],
                size: [400.0, 300.0],
                layers: vec![blank_layer(4)],
            },
        ],
    }
}

/// Premultiplied solid fill of `size`.
pub fn solid_layer_pixels((width, height): (u32, u32), rgba: [u8; 4]) -> Vec<u8> {
    let mut pixels = rgba.repeat(width as usize * height as usize);
    crate::document::loader::premultiply_alpha(&mut pixels);
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn doc_single_layer_has_a_drawable_layer() {
        let document = doc_single_layer();
        assert_eq!(document.artboards.len(), 1);
        assert_eq!(document.artboards[0].layers.len(), 1);
        assert!(document.artboards[0].layers[0].visible);
    }

    #[test]
    fn doc_two_artboards_is_well_formed() {
        let document = doc_two_artboards();
        assert_eq!(document.artboards.len(), 2);
        assert_ne!(
            document.artboards[0].position, document.artboards[1].position,
            "distinct world positions"
        );

        let mut ids = HashSet::new();
        for artboard in &document.artboards {
            assert!(ids.insert(artboard.id.0), "unique ids");
            assert!(artboard.id.0 < document.next_id);
            for layer in &artboard.layers {
                assert!(ids.insert(layer.id.0), "unique ids");
                assert!(layer.id.0 < document.next_id);
            }
        }
    }

    #[test]
    fn solid_layer_pixels_is_premultiplied() {
        let rgba = [255, 255, 255, 128];
        let mut pixels = rgba.repeat(3 * 2);
        crate::document::loader::premultiply_alpha(&mut pixels);

        assert_eq!(pixels.len(), 3 * 2 * 4);
        // Premultiplied alpha.
        assert!(pixels.chunks_exact(4).all(|px| px == [128, 128, 128, 128]));
    }
}
