use std::collections::{HashMap, HashSet};

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
use anyhow::bail;

use crate::document::{Document, LayerId};

pub struct LoadedDocument {
    pub document: Document,
    /// Decoded, premultiplied RGBA8 blocks
    /// Blank layers have no entry
    pub layer_pixels: HashMap<LayerId, Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_document(name: &str, max_texture_dim: u32) -> anyhow::Result<LoadedDocument> {
    let dir = asset_dir();
    let json_path = dir.join(format!("{name}.json"));
    let json = std::fs::read_to_string(&json_path)
        .with_context(|| format!("reading {}", json_path.display()))?;
    let mut document: Document =
        serde_json::from_str(&json).with_context(|| format!("parsing {}", json_path.display()))?;
    validate(&mut document, max_texture_dim)?;

    let mut layer_pixels = HashMap::new();
    for artboard in &document.artboards {
        let size = artboard.pixel_size();
        for layer in &artboard.layers {
            use anyhow::Context;

            let Some(content) = &layer.content_path else {
                continue;
            };
            let png_path = dir.join(content);
            let img = image::open(&png_path)
                .with_context(|| format!("decoding {}", png_path.display()))?
                .to_rgba8();

            let mut pixels = artboard_sized(&img, size);
            premultiply_alpha(&mut pixels);
            layer_pixels.insert(layer.id, pixels);
        }
    }
    Ok(LoadedDocument {
        document,
        layer_pixels,
    })
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::unused_async)]
pub async fn load_document(_name: &str, _max_texture_dim: u32) -> anyhow::Result<LoadedDocument> {
    todo!("WASM document fetch is slated for later")
}

/// Validates the document to be loaded with the following constraints:
/// - element ids are unique
/// - elements have valid sizes
/// - artboard dimensions are clamped to device specific max texture dims
fn validate(document: &mut Document, max_texture_dim: u32) -> anyhow::Result<()> {
    let max_dim = max_texture_dim as f32;
    let mut seen = HashSet::new();
    for artboard in &mut document.artboards {
        if !seen.insert(artboard.id.0) {
            bail!("duplicate id {} in document", artboard.id.0);
        }
        for (axis, extent) in artboard.size.iter_mut().enumerate() {
            if !extent.is_finite() || *extent < 1.0 {
                bail!(
                    "artboard {} has invalid size on axis {axis}: {extent}",
                    artboard.id.0
                );
            }
            *extent = extent.min(max_dim);
        }
        for layer in &artboard.layers {
            if !seen.insert(layer.id.0) {
                bail!("duplicate id {} in document", layer.id.0);
            }
        }
    }
    Ok(())
}

/// Crop/pad `img` into a `(width, height)` RGBA8 buffer with transparent padding anchored at the top-left.
fn artboard_sized(img: &image::RgbaImage, (width, height): (u32, u32)) -> Vec<u8> {
    let mut pixels_rgba = vec![0u8; width as usize * height as usize * 4];
    let copy_width = img.width().min(width) as usize * 4;
    let src_stride = img.width() as usize * 4;
    let dst_stride = width as usize * 4;
    let src = img.as_raw();

    for row in 0..img.height().min(height) as usize {
        let dst_start = row * dst_stride;
        let src_start = row * src_stride;

        pixels_rgba[dst_start..dst_start + copy_width]
            .copy_from_slice(&src[src_start..src_start + copy_width]);
    }

    pixels_rgba
}

/// Convert straight-alpha RGBA8 to premultiplied alpha in place.
pub fn premultiply_alpha(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(px[3]);
        for channel in &mut px[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
}

/// Returns the asset dir for bundled assets.
#[cfg(not(target_arch = "wasm32"))]
fn asset_dir() -> std::path::PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let bundled = exe_dir.join("assets/documents");
        if bundled.is_dir() {
            return bundled;
        }
    }

    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/documents"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::{Artboard, ArtboardId, DOCUMENT_VERSION, Layer};
    use super::*;

    fn two_layer_doc() -> Document {
        Document {
            version: DOCUMENT_VERSION,
            next_id: 3,
            artboards: vec![Artboard {
                id: ArtboardId(1),
                name: "A".to_string(),
                position: [0.0, 0.0],
                size: [100.0, 100.0],
                layers: vec![Layer {
                    id: LayerId(2),
                    name: "L".to_string(),
                    offset: [0.0, 0.0],
                    visible: true,
                    content_path: None,
                    thumbhash: None,
                }],
            }],
        }
    }

    #[test]
    fn validate_bails_on_duplicate_ids() {
        let mut document = two_layer_doc();
        // collides with artboard id
        document.artboards[0].layers[0].id = LayerId(1);
        assert!(validate(&mut document, 2048).is_err());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn validate_clamps_oversized_artboards() {
        let mut document = two_layer_doc();
        document.artboards[0].size = [5000.0, 1000.0];
        validate(&mut document, 2048).unwrap();
        assert_eq!(document.artboards[0].size, [2048.0, 1000.0]);
    }

    #[test]
    fn validate_bails_on_degenerate_size() {
        let mut document = two_layer_doc();
        document.artboards[0].size = [0.0, 100.0];
        assert!(validate(&mut document, 2048).is_err());
        document.artboards[0].size = [f32::NAN, 100.0];
        assert!(validate(&mut document, 2048).is_err());
    }

    #[test]
    fn artboard_sized_crops_and_pads() {
        // 3x2 source, red pixels, into a 2x3 target: crop x, pad y.
        let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([255, 0, 0, 255]));
        let pixels = artboard_sized(&img, (2, 3));
        assert_eq!(pixels.len(), 2 * 3 * 4);
        // copied
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
        // padded row
        assert_eq!(&pixels[2 * 2 * 4..2 * 2 * 4 + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn premultiply_scales_color_by_alpha() {
        let mut pixels = vec![255, 255, 255, 128, 200, 100, 0, 0];
        premultiply_alpha(&mut pixels);
        assert_eq!(&pixels[0..4], &[128, 128, 128, 128]);
        // zero alpha zeroes color
        assert_eq!(&pixels[4..8], &[0, 0, 0, 0]);
    }

    #[test]
    fn load_default_document_from_assets() {
        let loaded = load_document("default", 2048).unwrap();
        assert!(!loaded.document.artboards.is_empty());
        let mut content_layers = 0;
        for artboard in &loaded.document.artboards {
            let (w, h) = artboard.pixel_size();
            for layer in &artboard.layers {
                if layer.content_path.is_some() {
                    content_layers += 1;
                    assert!(layer.thumbhash.is_some(), "content layers carry a hash");
                    let pixels = loaded.layer_pixels.get(&layer.id).unwrap();
                    assert_eq!(pixels.len(), w as usize * h as usize * 4);
                    // Premultiplied: no channel may exceed alpha.
                    assert!(
                        pixels
                            .chunks_exact(4)
                            .all(|px| { px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3] })
                    );
                } else {
                    assert!(!loaded.layer_pixels.contains_key(&layer.id));
                }
            }
        }
        assert!(
            content_layers > 0,
            "default.json must exercise the PNG path"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn load_two_boards_document_from_assets() {
        let loaded = load_document("two-boards", 2048).unwrap();
        assert!(loaded.document.artboards.len() >= 2);
        let positions: Vec<_> = loaded
            .document
            .artboards
            .iter()
            .map(|artboard| artboard.position)
            .collect();
        assert_ne!(positions[0], positions[1], "distinct world positions");
    }

    /// Regenerates the committed test assets deterministically. Run manually:
    /// `cargo test -p crayon --lib generate_default_assets -- --ignored`
    #[test]
    #[ignore = "asset generator, not a test"]
    #[allow(clippy::too_many_lines)]
    fn generate_default_assets() {
        use super::super::thumbhash::generate_thumbhash;

        let dir =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/documents"));
        std::fs::create_dir_all(&dir).unwrap();

        // default.layer-2.png: 800x600, straight alpha,
        // transparent background, an opaque disc and a soft-edged diagonal band.
        let (w, h) = (800u32, 600u32);
        let mut img = image::RgbaImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            #[allow(clippy::cast_precision_loss)]
            let (fx, fy) = (x as f32, y as f32);
            // Opaque disc centered at (400, 300), radius 180.
            let d = ((fx - 400.0).powi(2) + (fy - 300.0).powi(2)).sqrt();
            if d <= 180.0 {
                *px = image::Rgba([228, 87, 46, 255]);
                continue;
            }
            // Diagonal band with alpha fading across its width.
            let band = (fx + fy - 500.0).abs();
            if band <= 60.0 {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let alpha = (255.0 * (1.0 - band / 60.0)).round() as u8;
                *px = image::Rgba([46, 134, 171, alpha]);
            }
        }
        img.save(dir.join("default.layer-2.png")).unwrap();
        let hash = generate_thumbhash(img.as_raw(), w, h).unwrap();

        let default_doc = Document {
            version: DOCUMENT_VERSION,
            next_id: 5,
            artboards: vec![
                Artboard {
                    id: ArtboardId(1),
                    name: "Artboard 1".to_string(),
                    position: [0.0, 0.0],
                    size: [800.0, 600.0],
                    layers: vec![
                        Layer {
                            id: LayerId(2),
                            name: "Background".to_string(),
                            offset: [0.0, 0.0],
                            visible: true,
                            content_path: Some("default.layer-2.png".to_string()),
                            thumbhash: Some(hash),
                        },
                        Layer {
                            id: LayerId(3),
                            name: "Sketch".to_string(),
                            offset: [24.0, 10.0],
                            visible: true,
                            content_path: None,
                            thumbhash: None,
                        },
                    ],
                },
                Artboard {
                    id: ArtboardId(4),
                    name: "Artboard 2".to_string(),
                    position: [880.0, 120.0],
                    size: [400.0, 400.0],
                    layers: Vec::new(),
                },
            ],
        };
        std::fs::write(
            dir.join("default.json"),
            serde_json::to_string_pretty(&default_doc).unwrap(),
        )
        .unwrap();

        // two-boards.json: blank layers only, distinct world positions.
        let two_boards = Document {
            version: DOCUMENT_VERSION,
            next_id: 5,
            artboards: vec![
                Artboard {
                    id: ArtboardId(1),
                    name: "Left".to_string(),
                    position: [0.0, 0.0],
                    size: [600.0, 400.0],
                    layers: vec![Layer {
                        id: LayerId(2),
                        name: "Layer 1".to_string(),
                        offset: [0.0, 0.0],
                        visible: true,
                        content_path: None,
                        thumbhash: None,
                    }],
                },
                Artboard {
                    id: ArtboardId(3),
                    name: "Right".to_string(),
                    position: [700.0, 100.0],
                    size: [400.0, 300.0],
                    layers: vec![Layer {
                        id: LayerId(4),
                        name: "Layer 1".to_string(),
                        offset: [0.0, 0.0],
                        visible: true,
                        content_path: None,
                        thumbhash: None,
                    }],
                },
            ],
        };
        std::fs::write(
            dir.join("two-boards.json"),
            serde_json::to_string_pretty(&two_boards).unwrap(),
        )
        .unwrap();
    }
}
