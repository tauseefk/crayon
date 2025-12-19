use std::ops::RangeInclusive;

use egui::{Color32, CornerRadius, Pos2, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::renderer::ui::theme::DEFAULT_THEME;

#[derive(Clone, Copy, PartialEq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

/// A styled slider with trailing fill
pub struct StyledSlider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    orientation: SliderOrientation,
    length: f32,
    thickness: f32,
    handle_radius: f32,
    step: Option<f64>,
}

impl<'a> StyledSlider<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            orientation: SliderOrientation::Horizontal,
            length: 200.0,
            thickness: 8.0,
            handle_radius: 12.0,
            step: None,
        }
    }

    pub fn vertical(mut self) -> Self {
        self.orientation = SliderOrientation::Vertical;
        self
    }

    #[allow(dead_code)]
    pub fn length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    #[allow(dead_code)]
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    #[allow(dead_code)]
    pub fn handle_radius(mut self, radius: f32) -> Self {
        self.handle_radius = radius;
        self
    }

    pub fn step_by(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    fn normalize(&self, value: f32) -> f32 {
        let min = *self.range.start();
        let max = *self.range.end();
        (value - min) / (max - min)
    }

    fn denormalize(&self, t: f32) -> f32 {
        let min = *self.range.start();
        let max = *self.range.end();
        min + t * (max - min)
    }
}

impl Widget for StyledSlider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = match self.orientation {
            SliderOrientation::Horizontal => Vec2::new(self.length, self.handle_radius * 2.0 + 4.0),
            SliderOrientation::Vertical => Vec2::new(self.handle_radius * 2.0 + 4.0, self.length),
        };

        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        let old_value = *self.value;

        if response.dragged() || response.clicked() {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
                let t = match self.orientation {
                    SliderOrientation::Horizontal => {
                        let x = (pointer_pos.x - rect.left()) / rect.width();
                        x.clamp(0.0, 1.0)
                    }
                    SliderOrientation::Vertical => {
                        // Invert for vertical (top = max, bottom = min)
                        let y = (rect.bottom() - pointer_pos.y) / rect.height();
                        y.clamp(0.0, 1.0)
                    }
                };

                let mut new_value = self.denormalize(t);

                if let Some(step) = self.step {
                    let step = step as f32;
                    new_value = (new_value / step).round() * step;
                }

                *self.value = new_value.clamp(*self.range.start(), *self.range.end());
            }
        }

        if ui.is_rect_visible(rect) {
            let theme = &DEFAULT_THEME;
            let painter = ui.painter();

            let t = self.normalize(*self.value);

            // Draw rail
            let rail_rect = match self.orientation {
                SliderOrientation::Horizontal => {
                    let center_y = rect.center().y;
                    egui::Rect::from_min_max(
                        Pos2::new(
                            rect.left() + self.handle_radius,
                            center_y - self.thickness / 2.0,
                        ),
                        Pos2::new(
                            rect.right() - self.handle_radius,
                            center_y + self.thickness / 2.0,
                        ),
                    )
                }
                SliderOrientation::Vertical => {
                    let center_x = rect.center().x;
                    egui::Rect::from_min_max(
                        Pos2::new(
                            center_x - self.thickness / 2.0,
                            rect.top() + self.handle_radius,
                        ),
                        Pos2::new(
                            center_x + self.thickness / 2.0,
                            rect.bottom() - self.handle_radius,
                        ),
                    )
                }
            };

            let rail_rounding = CornerRadius::same((self.thickness / 2.0) as u8);

            painter.rect_filled(rail_rect, rail_rounding, theme.surface_variant);
            painter.rect_stroke(
                rail_rect,
                rail_rounding,
                Stroke::new(1.0, theme.outline_variant),
                egui::StrokeKind::Middle,
            );

            // Trailing fill
            let fill_rect = match self.orientation {
                SliderOrientation::Horizontal => egui::Rect::from_min_max(
                    rail_rect.min,
                    Pos2::new(rail_rect.left() + t * rail_rect.width(), rail_rect.bottom()),
                ),
                SliderOrientation::Vertical => egui::Rect::from_min_max(
                    Pos2::new(
                        rail_rect.left(),
                        rail_rect.bottom() - t * rail_rect.height(),
                    ),
                    rail_rect.max,
                ),
            };
            painter.rect_filled(fill_rect, rail_rounding, theme.primary);

            let handle_center = match self.orientation {
                SliderOrientation::Horizontal => {
                    Pos2::new(rail_rect.left() + t * rail_rect.width(), rect.center().y)
                }
                SliderOrientation::Vertical => {
                    Pos2::new(rect.center().x, rail_rect.bottom() - t * rail_rect.height())
                }
            };

            // Draw handle
            let handle_color = if response.dragged() {
                theme.primary
            } else if response.hovered() {
                theme.primary_container
            } else {
                Color32::WHITE
            };

            painter.circle_filled(handle_center, self.handle_radius, handle_color);
            painter.circle_stroke(handle_center, self.handle_radius, Stroke::NONE);
        }

        if *self.value != old_value {
            response.mark_changed();
        }

        response
    }
}
