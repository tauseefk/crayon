use batteries::prelude::{Dot2D, screen_to_world_position, world_to_screen_position};
use cgmath::Point2;

use crate::{
    app::{App, WindowResource},
    event_sender::EventSender,
    events::ControllerEvent,
    prelude::Resource,
    renderer::ui::{drawable::Drawable, hello_points::HELLO_POINTS},
    resource::ResourceContext,
    state::State,
    utils::transform_point::transform_point,
};

pub struct HelloResource {
    point_idx: usize,
    is_animating: bool,
}

impl Resource for HelloResource {}

impl HelloResource {
    pub fn new() -> Self {
        Self {
            point_idx: 0,
            is_animating: false,
        }
    }

    fn get_point_and_increment(&mut self) -> Option<Point2<f32>> {
        let mut output = None;
        if self.is_animating && self.point_idx < HELLO_POINTS.len() {
            output = Some(HELLO_POINTS[self.point_idx]);
            self.point_idx += 1;
        }

        output
    }
}

pub struct HelloWidget;

impl HelloWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Drawable for HelloWidget {
    fn draw(&self, ctx: &egui::Context, app: &App) {
        let (Some(mut hello_res), Some(event_sender), Some(window), Some(state)) = (
            app.write::<HelloResource>(),
            app.read::<EventSender>(),
            app.read::<WindowResource>(),
            app.read::<State>(),
        ) else {
            return;
        };

        egui::Window::new("Intro")
            .fixed_pos(egui::pos2(8.0, 44.0))
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgb(216, 225, 255))
                    .shadow(egui::epaint::Shadow::NONE),
            )
            .show(ctx, |ui| {
                if ui
                    .add_sized([40.0, 20.0], egui::Button::new("👋"))
                    .clicked()
                {
                    hello_res.point_idx = 0;
                    hello_res.is_animating = true;
                }
            });

        if hello_res.is_animating {
            for _ in 0..20 {
                if let Some(point) = hello_res.get_point_and_increment() {
                    let window_size = window.0.inner_size();
                    #[allow(clippy::cast_precision_loss)]
                    let window_size = (window_size.width as f32, window_size.height as f32);

                    let position = screen_to_world_position(point, window_size);
                    let position = transform_point(position, &state.camera);
                    let position = world_to_screen_position(position, window_size);

                    event_sender.send(ControllerEvent::BrushPoint {
                        dot: Dot2D {
                            position,
                            radius: 0.06668,
                        },
                    });
                }
            }
        }
    }
}
