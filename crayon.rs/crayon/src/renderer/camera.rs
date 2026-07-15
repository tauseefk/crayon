use cgmath::{EuclideanSpace, Point2, Vector2};

use crate::{constants::DEFAULT_CANVAS_ZOOM, utils::clamp};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // can't use cgmath with bytemuck directly
    // so convert the Matrix4 into a 4x4 f32 array
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_projection: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_projection(&mut self, camera: &Camera2D) {
        self.view_projection = camera.world_to_clip_matrix().into();
    }
}

/// Axis-aligned rectangle in world px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldRect {
    pub min: Point2<f32>,
    pub max: Point2<f32>,
}

impl WorldRect {
    pub fn from_origin_size(origin: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            min: Point2::new(origin[0], origin[1]),
            max: Point2::new(origin[0] + size[0], origin[1] + size[1]),
        }
    }

    pub fn intersects(&self, other: &WorldRect) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// 2D camera over the world-px scene.
///
/// Coordinate spaces (multi-artboard.md §2.2), all y-down except clip space:
///
/// | Space          | Units | Definition                                             |
/// |----------------|-------|--------------------------------------------------------|
/// | World          | px    | Origin arbitrary; artboard `position`/`size` live here |
/// | Artboard-local | px    | `world - artboard.position`                            |
/// | Layer-local    | px    | `artboard_local - layer.offset` (= layer texel space)  |
/// | Clip/NDC       | —     | Produced only by this camera                           |
#[derive(Clone, Copy)]
pub struct Camera2D {
    /// World-px → screen-px zoom factor; the same in both axes.
    scale: f32,
    /// Render-target size in px.
    viewport: (f32, f32),
    /// The world-px point at the viewport center.
    translation: Point2<f32>,
}

impl Camera2D {
    pub fn with_viewport(width: f32, height: f32) -> Self {
        Self {
            scale: DEFAULT_CANVAS_ZOOM,
            viewport: (width, height),
            translation: Point2::origin(),
        }
    }

    /// Moves the viewport center to `world`.
    pub fn center_on(&mut self, world: Point2<f32>) {
        self.translation = world;
    }

    pub fn zoom_by(&mut self, delta: f32) {
        self.scale = clamp::clamp_zoom(self.scale, delta);
    }

    /// Pans by a screen-px drag delta so content follows the cursor 1:1 at any zoom.
    pub fn pan_screen_delta(&mut self, delta: Vector2<f32>) {
        self.translation -= delta / self.scale;
    }

    /// World px are resize-invariant: a resize only changes how much world is visible.
    pub fn update_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
    }

    /// `clip.x =  2*scale/vw * (world.x - translation.x)`
    /// `clip.y = -2*scale/vh * (world.y - translation.y)` — the y flip lives here.
    pub fn world_to_clip_matrix(&self) -> cgmath::Matrix4<f32> {
        let scale_matrix = cgmath::Matrix4::from_nonuniform_scale(
            2.0 * self.scale / self.viewport.0,
            -2.0 * self.scale / self.viewport.1,
            1.0,
        );

        let translation_matrix = cgmath::Matrix4::from_translation(cgmath::Vector3::new(
            -self.translation.x,
            -self.translation.y,
            0.0,
        ));

        // order dependent: translate into camera space, then scale to clip
        scale_matrix * translation_matrix
    }

    /// Consumed by the paint stage (S3) to place brush points in world space.
    #[allow(dead_code)]
    pub fn screen_to_world(&self, screen: Point2<f32>) -> Point2<f32> {
        self.translation + (screen - self.viewport_center()) / self.scale
    }

    pub fn world_to_screen(&self, world: Point2<f32>) -> Point2<f32> {
        self.viewport_center() + (world - self.translation) * self.scale
    }

    /// The world-px rect currently visible in the viewport, for culling.
    pub fn visible_world_rect(&self) -> WorldRect {
        let half_extent =
            Vector2::new(self.viewport.0, self.viewport.1) / (2.0 * self.scale);
        WorldRect {
            min: self.translation - half_extent,
            max: self.translation + half_extent,
        }
    }

    fn viewport_center(&self) -> Point2<f32> {
        Point2::new(self.viewport.0 / 2.0, self.viewport.1 / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{Point3, Transform};

    fn camera(scale_delta: f32, center: Point2<f32>) -> Camera2D {
        let mut camera = Camera2D::with_viewport(800.0, 600.0);
        camera.zoom_by(scale_delta);
        camera.center_on(center);
        camera
    }

    fn project(camera: &Camera2D, world: Point2<f32>) -> Point2<f32> {
        let clip = camera
            .world_to_clip_matrix()
            .transform_point(Point3::new(world.x, world.y, 0.0));
        Point2::new(clip.x, clip.y)
    }

    #[test]
    fn viewport_center_maps_to_clip_origin() {
        let camera = camera(0.0, Point2::new(100.0, 50.0));
        let clip = project(&camera, Point2::new(100.0, 50.0));
        assert!(clip.x.abs() < 1e-6 && clip.y.abs() < 1e-6);
    }

    #[test]
    fn world_to_clip_flips_y_and_scales_per_axis() {
        // scale 1: the visible world spans 800x600 around the center.
        let camera = camera(0.0, Point2::new(0.0, 0.0));
        let right_edge = project(&camera, Point2::new(400.0, 0.0));
        assert!((right_edge.x - 1.0).abs() < 1e-6);
        // +y in world (down) maps to -y in clip.
        let bottom_edge = project(&camera, Point2::new(0.0, 300.0));
        assert!((bottom_edge.y + 1.0).abs() < 1e-6);
    }

    #[test]
    fn screen_world_round_trip() {
        let camera = camera(1.0, Point2::new(123.0, -45.0)); // scale 2.0
        let screen = Point2::new(700.0, 20.0);
        let world = camera.screen_to_world(screen);
        let back = camera.world_to_screen(world);
        assert!((back.x - screen.x).abs() < 1e-3 && (back.y - screen.y).abs() < 1e-3);
        // At scale 2, 300 screen px right of center is 150 world px.
        assert!((world.x - (123.0 + 150.0)).abs() < 1e-3);
    }

    #[test]
    fn pan_follows_cursor_one_to_one() {
        let mut camera = camera(1.0, Point2::new(0.0, 0.0)); // scale 2.0
        let anchor = camera.world_to_screen(Point2::new(10.0, 10.0));
        camera.pan_screen_delta(Vector2::new(40.0, -20.0));
        let moved = camera.world_to_screen(Point2::new(10.0, 10.0));
        assert!((moved.x - (anchor.x + 40.0)).abs() < 1e-3);
        assert!((moved.y - (anchor.y - 20.0)).abs() < 1e-3);
    }

    #[test]
    fn visible_world_rect_matches_viewport_over_scale() {
        let camera = camera(1.0, Point2::new(100.0, 100.0)); // scale 2.0
        let rect = camera.visible_world_rect();
        assert!((rect.min.x - (100.0 - 200.0)).abs() < 1e-3);
        assert!((rect.max.x - (100.0 + 200.0)).abs() < 1e-3);
        assert!((rect.min.y - (100.0 - 150.0)).abs() < 1e-3);
        assert!((rect.max.y - (100.0 + 150.0)).abs() < 1e-3);
    }

    #[test]
    fn zoom_clamps_to_bounds() {
        let mut camera = Camera2D::with_viewport(800.0, 600.0);
        camera.zoom_by(100.0);
        let max_rect = camera.visible_world_rect();
        camera.zoom_by(100.0);
        assert_eq!(camera.visible_world_rect(), max_rect, "zoom clamped at max");
        camera.zoom_by(-100.0);
        let min_rect = camera.visible_world_rect();
        camera.zoom_by(-100.0);
        assert_eq!(camera.visible_world_rect(), min_rect, "zoom clamped at min");
    }

    #[test]
    fn rect_intersection() {
        let a = WorldRect::from_origin_size([0.0, 0.0], [100.0, 100.0]);
        let b = WorldRect::from_origin_size([50.0, 50.0], [100.0, 100.0]);
        let c = WorldRect::from_origin_size([200.0, 0.0], [10.0, 10.0]);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
        // Touching edges do not intersect (empty overlap).
        let d = WorldRect::from_origin_size([100.0, 0.0], [10.0, 10.0]);
        assert!(!a.intersects(&d));
    }
}
