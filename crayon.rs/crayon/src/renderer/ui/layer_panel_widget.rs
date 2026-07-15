use std::collections::HashMap;
use std::sync::Mutex;

use cgmath::Point2;

use crate::{
    app::App,
    document::{Artboard, Layer, LayerId, thumbhash::thumbhash_preview},
    event_sender::EventSender,
    events::ControllerEvent,
    renderer::ui::{
        drawable::Drawable,
        theme::{
            DEFAULT_THEME,
            widgets::{GLOBAL_PADDING, PillButton},
        },
    },
    resource::ResourceContext,
    resources::document_state::DocumentState,
    state::State,
};

const PANEL_WIDTH: f32 = 220.0;
const PREVIEW_SIZE: egui::Vec2 = egui::Vec2::new(32.0, 24.0);
const OUTLINE_STROKE: f32 = 2.0;
/// Small square action buttons (`+`, `×`, visibility toggle).
const ACTION_BUTTON_SIZE: egui::Vec2 = egui::Vec2::splat(24.0);
const ACTION_BUTTON_ROUNDING: f32 = 6.0;
/// Vertical gap between the panel's rows.
const ROW_GAP: f32 = 8.0;

/// Right side panel (multi-artboard.md §4): the artboard list and, for the
/// selected artboard, its layer list — with thumbhash previews and
/// select/visibility/add/delete controls. All mutations go through
/// `ControllerEvent`s; the widget never writes `DocumentState` (§6).
pub struct LayerPanelWidget {
    /// Thumbhash preview textures cached per layer. `Drawable::draw` takes
    /// `&self`, hence the mutex; it is uncontended (only `ToolsSystem`
    /// draws). Hashes only change at save, so entries never go stale —
    /// they are only pruned when their layer is deleted.
    previews: Mutex<HashMap<LayerId, egui::TextureHandle>>,
}

impl LayerPanelWidget {
    pub fn new() -> Self {
        Self {
            previews: Mutex::new(HashMap::new()),
        }
    }
}

impl Drawable for LayerPanelWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let (Some(event_sender), Some(doc)) =
            (app.read::<EventSender>(), app.read::<DocumentState>())
        else {
            return;
        };
        let Ok(mut previews) = self.previews.lock() else {
            return;
        };
        previews.retain(|id, _| doc.document.find_layer(*id).is_some());

        let selected_artboard = doc.selection.selected_artboard();
        let selected_layer = doc.selection.selected_layer().map(|(_, layer)| layer);

        egui::SidePanel::right("layers")
            .default_width(PANEL_WIDTH)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(GLOBAL_PADDING))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = ROW_GAP;

                section_header(ui, "Artboards", || {
                    event_sender.send(ControllerEvent::AddArtboard);
                });
                if doc.document.artboards.is_empty() {
                    ui.weak("No artboards");
                }
                for artboard in &doc.document.artboards {
                    artboard_row(
                        ui,
                        ctx,
                        &mut previews,
                        artboard,
                        selected_artboard == Some(artboard.id),
                        &event_sender,
                    );
                }

                ui.add_space(12.0);

                if let Some(artboard) =
                    selected_artboard.and_then(|id| doc.document.artboard(id))
                {
                    section_header(ui, "Layers", || {
                        event_sender.send(ControllerEvent::AddLayer(artboard.id));
                    });
                    if artboard.layers.is_empty() {
                        ui.weak("No layers");
                    }
                    // Panel order is top-to-bottom; layers are stored
                    // bottom-to-top.
                    for layer in artboard.layers.iter().rev() {
                        layer_row(
                            ui,
                            ctx,
                            &mut previews,
                            artboard,
                            layer,
                            selected_layer == Some(layer.id),
                            &event_sender,
                        );
                    }
                }
            });

        // Selected-artboard outline over the canvas, world→screen through
        // the camera (screen px → egui points via pixels_per_point).
        if let (Some(artboard_id), Some(state)) = (selected_artboard, app.read::<State>())
            && let Some(artboard) = doc.document.artboard(artboard_id)
        {
            let points_per_pixel = ctx.pixels_per_point().recip();
            let min = state.camera.world_to_screen(Point2::new(
                artboard.position[0],
                artboard.position[1],
            ));
            let max = state.camera.world_to_screen(Point2::new(
                artboard.position[0] + artboard.size[0],
                artboard.position[1] + artboard.size[1],
            ));
            let rect = egui::Rect::from_min_max(
                egui::pos2(min.x * points_per_pixel, min.y * points_per_pixel),
                egui::pos2(max.x * points_per_pixel, max.y * points_per_pixel),
            );
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("artboard_outline"),
            ));
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(OUTLINE_STROKE, DEFAULT_THEME.primary),
                egui::StrokeKind::Outside,
            );
        }
    }
}

/// Small square rounded-rect button (`+`, `×`, visibility toggle).
fn action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        PillButton::new(label)
            .min_size(ACTION_BUTTON_SIZE)
            .padding(egui::Vec2::splat(4.0))
            .corner_radius(ACTION_BUTTON_ROUNDING),
    )
}

/// Heading with a right-aligned `+` button.
fn section_header(ui: &mut egui::Ui, title: &str, on_add: impl FnOnce()) {
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_button(ui, "+").clicked() {
                on_add();
            }
        });
    });
    ui.separator();
}

fn artboard_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    previews: &mut HashMap<LayerId, egui::TextureHandle>,
    artboard: &Artboard,
    selected: bool,
    event_sender: &EventSender,
) {
    ui.horizontal(|ui| {
        // Preview of the topmost layer stands in for the artboard.
        preview_slot(ui, ctx, previews, artboard.layers.last());
        if ui
            .add(PillButton::new(&artboard.name).selected(selected))
            .clicked()
        {
            event_sender.send(ControllerEvent::SelectArtboard(artboard.id));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_button(ui, "×").clicked() {
                event_sender.send(ControllerEvent::DeleteArtboard(artboard.id));
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn layer_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    previews: &mut HashMap<LayerId, egui::TextureHandle>,
    artboard: &Artboard,
    layer: &Layer,
    selected: bool,
    event_sender: &EventSender,
) {
    ui.horizontal(|ui| {
        let eye = if layer.visible { "●" } else { "○" };
        if action_button(ui, eye)
            .on_hover_text("Toggle visibility")
            .clicked()
        {
            event_sender.send(ControllerEvent::ToggleLayerVisibility(layer.id));
        }
        preview_slot(ui, ctx, previews, Some(layer));
        if ui
            .add(PillButton::new(&layer.name).selected(selected))
            .clicked()
        {
            event_sender.send(ControllerEvent::SelectLayer(artboard.id, layer.id));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if action_button(ui, "×").clicked() {
                event_sender.send(ControllerEvent::DeleteLayer(layer.id));
            }
        });
    });
}

/// Fixed-size preview: the layer's decoded thumbhash, or an empty framed
/// rect for blank layers (and layerless artboards) so rows stay aligned.
fn preview_slot(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    previews: &mut HashMap<LayerId, egui::TextureHandle>,
    layer: Option<&Layer>,
) {
    if let Some(texture) = layer.and_then(|layer| preview_texture(ctx, previews, layer)) {
        ui.add(egui::Image::new(&texture).fit_to_exact_size(PREVIEW_SIZE));
    } else {
        let (rect, _) = ui.allocate_exact_size(PREVIEW_SIZE, egui::Sense::hover());
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, DEFAULT_THEME.outline_variant),
            egui::StrokeKind::Inside,
        );
    }
}

/// Cached thumbhash preview; `None` for blank layers or undecodable hashes.
fn preview_texture(
    ctx: &egui::Context,
    previews: &mut HashMap<LayerId, egui::TextureHandle>,
    layer: &Layer,
) -> Option<egui::TextureHandle> {
    if let Some(texture) = previews.get(&layer.id) {
        return Some(texture.clone());
    }
    let hash = layer.thumbhash.as_ref()?;
    let (width, height, rgba) = thumbhash_preview(hash).ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba);
    let texture = ctx.load_texture(
        format!("thumbhash-{}", layer.id.0),
        image,
        egui::TextureOptions::LINEAR,
    );
    previews.insert(layer.id, texture.clone());
    Some(texture)
}
