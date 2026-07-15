use std::collections::{HashMap, HashSet};

use anyhow::{Context, bail};

use super::{Document, LayerId};

pub struct LoadedDocument {
    pub document: Document,
    /// Decoded, PREMULTIPLIED RGBA8 blocks keyed by artboard-sized layer.
    /// Blank layers have no entry (hydration clears to TRANSPARENT).
    pub layer_pixels: HashMap<LayerId, Vec<u8>>,
}

/// The strict boot path: any unreadable content fails the whole load, and
/// the caller falls back to the default document.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_document(name: &str, max_texture_dim: u32) -> anyhow::Result<LoadedDocument> {
    load_from_json_path(
        &assets_dir().join(format!("{name}.json")),
        max_texture_dim,
        true,
    )
}

/// Load a document picked at runtime (§1.9); content PNGs resolve relative
/// to the JSON's directory. Tolerant: an unreadable PNG degrades that layer
/// to its thumbhash placeholder instead of failing the open.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_document_from_path(
    json_path: &std::path::Path,
    max_texture_dim: u32,
) -> anyhow::Result<LoadedDocument> {
    load_from_json_path(json_path, max_texture_dim, false)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_from_json_path(
    json_path: &std::path::Path,
    max_texture_dim: u32,
    strict: bool,
) -> anyhow::Result<LoadedDocument> {
    let json = std::fs::read_to_string(json_path)
        .with_context(|| format!("reading {}", json_path.display()))?;
    let mut document: Document =
        serde_json::from_str(&json).with_context(|| format!("parsing {}", json_path.display()))?;
    validate(&mut document, max_texture_dim)?;

    let dir = json_path.parent().unwrap_or(std::path::Path::new("."));
    let layer_pixels = decode_content_layers(&document, strict, |content| {
        let png_path = dir.join(content);
        std::fs::read(&png_path).with_context(|| format!("reading {}", png_path.display()))
    })?;

    Ok(LoadedDocument {
        document,
        layer_pixels,
    })
}

/// Load a document from an in-memory picked-file set — the web Open path
/// (§1.9), where a browser cannot follow relative paths from a picked file:
/// the selection holds exactly one `*.json` plus any of its content PNGs,
/// matched by file name. Target-independent so it is natively unit-testable;
/// the runtime caller is the wasm `OpenDocument` arm.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn load_document_from_files(
    files: &[(String, Vec<u8>)],
    max_texture_dim: u32,
) -> anyhow::Result<LoadedDocument> {
    let is_json = |name: &str| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    };
    let mut jsons = files.iter().filter(|(name, _)| is_json(name));
    let (json_name, json) = jsons.next().context("no .json file in the selection")?;
    if jsons.next().is_some() {
        bail!("more than one .json file in the selection");
    }

    let mut document: Document =
        serde_json::from_slice(json).with_context(|| format!("parsing {json_name}"))?;
    validate(&mut document, max_texture_dim)?;

    let layer_pixels = decode_content_layers(&document, false, |content| {
        // `content` may carry a relative dir; picked files are flat.
        let file_name = std::path::Path::new(content)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(content);
        files
            .iter()
            .find(|(name, _)| name == file_name)
            .map(|(_, bytes)| bytes.clone())
            .with_context(|| format!("{file_name} is not in the selection"))
    })?;

    Ok(LoadedDocument {
        document,
        layer_pixels,
    })
}

/// Decode every content layer through `read` (content ref → PNG bytes).
/// `strict` fails the load on the first unreadable content; tolerant callers
/// (the Open paths, §1.9) warn and leave the layer to render as its
/// thumbhash placeholder (§1.6).
fn decode_content_layers(
    document: &Document,
    strict: bool,
    mut read: impl FnMut(&str) -> anyhow::Result<Vec<u8>>,
) -> anyhow::Result<HashMap<LayerId, Vec<u8>>> {
    let mut layer_pixels = HashMap::new();
    for artboard in &document.artboards {
        let size = artboard.pixel_size();
        for layer in &artboard.layers {
            let Some(content) = &layer.content else {
                continue;
            };
            let decoded = read(content).and_then(|bytes| {
                image::load_from_memory(&bytes)
                    .map(|img| img.to_rgba8())
                    .map_err(|error| anyhow::anyhow!("decoding {content}: {error}"))
            });
            let img = match decoded {
                Ok(img) => img,
                Err(error) if strict => return Err(error),
                Err(error) => {
                    log::warn!(
                        "layer {} content unavailable: {error:#}; \
                         its thumbhash placeholder will render",
                        layer.id.0
                    );
                    continue;
                }
            };
            let mut pixels = artboard_sized(&img, size);
            premultiply(&mut pixels);
            layer_pixels.insert(layer.id, pixels);
        }
    }
    Ok(layer_pixels)
}

/// Fetches and validates `assets/documents/{name}.json` — the structural half
/// of the wasm load path (§1.7). PNG content arrives separately via
/// [`fetch_layer_pixels`], so the caller can hydrate thumbhash placeholders
/// from the document alone while the pixel fetches are in flight (§1.6).
#[cfg(target_arch = "wasm32")]
pub async fn fetch_document(name: &str, max_texture_dim: u32) -> anyhow::Result<Document> {
    let url = format!("./assets/documents/{name}.json");
    let json = fetch::text(&url).await?;
    let mut document: Document =
        serde_json::from_str(&json).map_err(|error| anyhow::anyhow!("parsing {url}: {error}"))?;
    validate(&mut document, max_texture_dim)?;
    Ok(document)
}

/// Fetches and decodes every content layer's PNG, artboard-sized and
/// premultiplied — same output contract as the native path.
#[cfg(target_arch = "wasm32")]
pub async fn fetch_layer_pixels(
    document: &Document,
) -> anyhow::Result<HashMap<LayerId, Vec<u8>>> {
    let mut layer_pixels = HashMap::new();
    for artboard in &document.artboards {
        let size = artboard.pixel_size();
        for layer in &artboard.layers {
            let Some(content) = &layer.content else {
                continue;
            };
            let url = format!("./assets/documents/{content}");
            let bytes = fetch::bytes(&url).await?;
            let img = image::load_from_memory(&bytes)
                .map_err(|error| anyhow::anyhow!("decoding {url}: {error}"))?
                .to_rgba8();
            let mut pixels = artboard_sized(&img, size);
            premultiply(&mut pixels);
            layer_pixels.insert(layer.id, pixels);
        }
    }
    Ok(layer_pixels)
}

/// `web_sys::fetch` wrappers. Errors cross the JS boundary as `JsValue`
/// (neither `Send` nor `Error`), so they are stringified into `anyhow`
/// immediately.
#[cfg(target_arch = "wasm32")]
mod fetch {
    use anyhow::Context;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::Response;

    fn js_error(action: &str, url: &str, value: &JsValue) -> anyhow::Error {
        anyhow::anyhow!("{action} {url}: {value:?}")
    }

    async fn response(url: &str) -> anyhow::Result<Response> {
        let window = web_sys::window().context("no window")?;
        let response = JsFuture::from(window.fetch_with_str(url))
            .await
            .map_err(|error| js_error("fetching", url, &error))?;
        let response: Response = response
            .dyn_into()
            .map_err(|value| js_error("fetching", url, &value))?;
        if !response.ok() {
            anyhow::bail!("fetching {url}: HTTP {}", response.status());
        }
        Ok(response)
    }

    pub async fn text(url: &str) -> anyhow::Result<String> {
        let response = response(url).await?;
        let text = JsFuture::from(
            response
                .text()
                .map_err(|error| js_error("reading", url, &error))?,
        )
        .await
        .map_err(|error| js_error("reading", url, &error))?;
        text.as_string()
            .with_context(|| format!("reading {url}: response text is not a string"))
    }

    pub async fn bytes(url: &str) -> anyhow::Result<Vec<u8>> {
        let response = response(url).await?;
        let buffer = JsFuture::from(
            response
                .array_buffer()
                .map_err(|error| js_error("reading", url, &error))?,
        )
        .await
        .map_err(|error| js_error("reading", url, &error))?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
}

/// Structural validation shared by every load path:
/// unique ids, sane sizes, artboard dimensions clamped to the device max texture dimension.
fn validate(document: &mut Document, max_texture_dim: u32) -> anyhow::Result<()> {
    #[allow(clippy::cast_precision_loss)]
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

/// Crop/pad `img` into a `(width, height)` RGBA8 buffer with transparent padding, anchored at the top-left.
fn artboard_sized(img: &image::RgbaImage, (width, height): (u32, u32)) -> Vec<u8> {
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let copy_width = img.width().min(width) as usize * 4;
    let src_stride = img.width() as usize * 4;
    let dst_stride = width as usize * 4;
    let src = img.as_raw();
    for row in 0..img.height().min(height) as usize {
        let dst_start = row * dst_stride;
        let src_start = row * src_stride;
        pixels[dst_start..dst_start + copy_width]
            .copy_from_slice(&src[src_start..src_start + copy_width]);
    }
    pixels
}

/// Convert straight-alpha RGBA8 to premultiplied alpha in place.
pub(crate) fn premultiply(pixels: &mut [u8]) {
    #[allow(clippy::cast_possible_truncation)]
    for px in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(px[3]);
        for channel in &mut px[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
}

/// `exe_dir/assets/documents` when running from a bundle, falling back to the
/// crate's own asset dir for `cargo run` / `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
fn assets_dir() -> std::path::PathBuf {
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
                    content: None,
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
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]); // copied
        assert_eq!(&pixels[2 * 2 * 4..2 * 2 * 4 + 4], &[0, 0, 0, 0]); // padded row
    }

    #[test]
    fn premultiply_scales_color_by_alpha() {
        let mut pixels = vec![255, 255, 255, 128, 200, 100, 0, 0];
        premultiply(&mut pixels);
        assert_eq!(&pixels[0..4], &[128, 128, 128, 128]);
        assert_eq!(&pixels[4..8], &[0, 0, 0, 0]); // zero alpha zeroes color
    }

    fn png_bytes(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(width, height, image::Rgba(rgba));
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    fn doc_json_with_content(content: &str) -> Vec<u8> {
        let mut document = two_layer_doc();
        document.artboards[0].layers[0].content = Some(content.to_string());
        serde_json::to_vec(&document).unwrap()
    }

    #[test]
    fn from_files_loads_json_plus_png() {
        let files = vec![
            ("doc.json".to_string(), doc_json_with_content("art.png")),
            ("art.png".to_string(), png_bytes(2, 2, [255, 0, 0, 255])),
        ];
        let loaded = load_document_from_files(&files, 2048).unwrap();
        let pixels = loaded.layer_pixels.get(&LayerId(2)).unwrap();
        // Artboard-sized (100x100), copied top-left, transparent padding.
        assert_eq!(pixels.len(), 100 * 100 * 4);
        assert_eq!(&pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&pixels[3 * 4..4 * 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn from_files_matches_content_by_file_name() {
        // `content` may carry a directory; picked files are flat.
        let files = vec![
            (
                "doc.json".to_string(),
                doc_json_with_content("images/art.png"),
            ),
            ("art.png".to_string(), png_bytes(2, 2, [255, 0, 0, 255])),
        ];
        let loaded = load_document_from_files(&files, 2048).unwrap();
        assert!(loaded.layer_pixels.contains_key(&LayerId(2)));
    }

    #[test]
    fn from_files_degrades_missing_png_to_placeholder() {
        let files = vec![("doc.json".to_string(), doc_json_with_content("art.png"))];
        let loaded = load_document_from_files(&files, 2048).unwrap();
        // Not an error: the layer renders as its thumbhash placeholder.
        assert!(loaded.layer_pixels.is_empty());
    }

    #[test]
    fn from_files_requires_exactly_one_json() {
        let none = vec![("a.png".to_string(), Vec::new())];
        assert!(load_document_from_files(&none, 2048).is_err());

        let two = vec![
            ("a.json".to_string(), doc_json_with_content("x.png")),
            ("b.json".to_string(), doc_json_with_content("x.png")),
        ];
        assert!(load_document_from_files(&two, 2048).is_err());
    }

    #[test]
    fn from_path_matches_the_boot_loader() {
        let path = std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/documents/default.json"
        ));
        let by_path = load_document_from_path(&path, 2048).unwrap();
        let by_name = load_document("default", 2048).unwrap();
        assert_eq!(by_path.document, by_name.document);
        assert_eq!(
            by_path.layer_pixels.keys().collect::<HashSet<_>>(),
            by_name.layer_pixels.keys().collect::<HashSet<_>>()
        );
    }

    #[test]
    fn load_default_document_from_assets() {
        let loaded = load_document("default", 2048).unwrap();
        assert!(!loaded.document.artboards.is_empty());
        let mut content_layers = 0;
        for artboard in &loaded.document.artboards {
            let (w, h) = artboard.pixel_size();
            for layer in &artboard.layers {
                if layer.content.is_some() {
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

        // --- default.layer-2.png: 800x600, straight alpha, transparent
        // background, an opaque disc and a soft-edged diagonal band.
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
                            content: Some("default.layer-2.png".to_string()),
                            thumbhash: Some(hash),
                        },
                        Layer {
                            id: LayerId(3),
                            name: "Sketch".to_string(),
                            offset: [24.0, 10.0],
                            visible: true,
                            content: None,
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

        // --- two-boards.json: blank layers only, distinct world positions.
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
                        content: None,
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
                        content: None,
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
