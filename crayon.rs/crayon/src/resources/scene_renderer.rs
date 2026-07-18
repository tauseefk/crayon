use batteries::prelude::AABB;

use crate::renderer::camera::Camera2D;

fn scissor_rect(
    camera: &Camera2D,
    rect: &AABB,
    (target_width, target_height): (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let (max_x, max_y) = (target_width as f32, target_height as f32);
    let min = camera.world_to_screen(rect.min);
    let max = camera.world_to_screen(rect.max);

    let (x0, y0, x1, y1) = (
        rect.min.x.floor().clamp(0.0, max_x) as u32,
        rect.min.y.floor().clamp(0.0, max_y) as u32,
        rect.max.x.ceil().clamp(0.0, max_x) as u32,
        rect.max.y.ceil().clamp(0.0, max_y) as u32,
    );

    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    Some((x0, y0, x1 - x0, y1 - y0))
}
