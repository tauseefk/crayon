use std::collections::{HashMap, HashSet};

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
    use anyhow::Context;

    let dir = asset_dir();
    let json_path = dir.join(format!("{name}.json"));
    let json = std::fs::read_to_string(&json_path)?;
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

fn premultiply_alpha(straight_alpha_pixels: &mut [u8]) {
    for px in straight_alpha_pixels.chunks_exact_mut(4) {
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
