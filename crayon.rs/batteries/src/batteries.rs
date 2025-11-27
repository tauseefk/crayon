use crate::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct Dot2D {
    pub position: Point2<f32>,
    pub radius: f32,
}

impl fmt::Display for Dot2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} {}]", self.position.x, self.position.y,)
    }
}

type Dot2Dx4 = [Dot2D; 4];

#[derive(Copy, Clone, Debug)]
pub struct StrokeDot2D {
    pub position: Point2<f32>,
    pub radius: f32,
    pub is_last: bool,
}

impl fmt::Display for StrokeDot2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.position.x, self.position.y,)
    }
}

impl From<StrokeDot2D> for Dot2D {
    fn from(value: StrokeDot2D) -> Self {
        Dot2D {
            position: value.position,
            radius: value.radius,
        }
    }
}

pub fn catmull_rom_to_bezier(dots: Dot2Dx4) -> Dot2Dx4 {
    let tension = 1.;
    let i6 = 1. / 6. / tension;

    let p0 = dots[0];
    let p1 = dots[1];
    let p2 = dots[2];
    let p3 = dots[3];

    [
        p1,
        Dot2D {
            position: Point2 {
                x: p2.position.x * i6 + p1.position.x - p0.position.x * i6,
                y: p2.position.y * i6 + p1.position.y - p0.position.y * i6,
            },
            radius: p2.radius * i6 + p1.radius - p0.radius * i6,
        },
        Dot2D {
            position: Point2 {
                x: p3.position.x * i6 + p2.position.x - p1.position.x * i6,
                y: p3.position.y * i6 + p2.position.y - p1.position.y * i6,
            },
            radius: p3.radius * i6 + p2.radius - p1.radius * i6,
        },
        p2,
    ]
}

pub fn eval_bezier(dots: Dot2Dx4, dots_count: usize) -> Vec<Dot2D> {
    let mut out_dots: Vec<Dot2D> = Vec::with_capacity(dots_count);

    let d0 = dots[0];
    let d1 = dots[1];
    let d2 = dots[2];
    let d3 = dots[3];

    for i in 0..dots_count {
        let k = (i) as f32 / (dots_count + 1) as f32;

        let d01 = lerp_dot_2d(d0, d1, k);
        let d12 = lerp_dot_2d(d1, d2, k);
        let d23 = lerp_dot_2d(d2, d3, k);

        let d012 = lerp_dot_2d(d01, d12, k);
        let d123 = lerp_dot_2d(d12, d23, k);

        let bez = lerp_dot_2d(d012, d123, k);

        out_dots.push(bez);
    }

    out_dots
}

/// Only allows points that are a minimum `distance` away from each other.
///
pub struct DistanceFilter {
    distance: f32,
    current_point: Option<Point2<f32>>,
}

impl DistanceFilter {
    pub fn new(distance: f32) -> Self {
        Self {
            distance,
            current_point: None,
        }
    }

    pub fn filter_by_distance(&mut self, next: Point2<f32>) -> Option<Point2<f32>> {
        match self.current_point {
            Some(current_point_val) => {
                let diff = current_point_val.sub_element_wise(next);
                let diff_sqr_len = sqr_len(diff);
                if diff_sqr_len > self.distance * self.distance {
                    self.current_point = Some(next);
                    self.current_point
                } else {
                    None
                }
            }
            None => {
                self.current_point = Some(next);
                self.current_point
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Point2;

    #[test]
    fn get_distance_filter_works_correctly() {
        let mut filter = DistanceFilter::new(10.0);
        let points = vec![
            Point2 { x: 0.0, y: 0.0 },
            Point2 { x: 10.0, y: 10.0 },
            Point2 { x: 12.0, y: 12.0 },
            Point2 { x: 13.0, y: 13.0 },
            Point2 { x: 40.0, y: 40.0 },
        ];

        let filtered_points: Vec<_> = points
            .into_iter()
            .filter_map(|point| filter.filter_by_distance(point))
            .collect();
        assert_eq!(
            filtered_points,
            vec![
                Point2 { x: 0.0, y: 0.0 },
                Point2 { x: 10.0, y: 10.0 },
                Point2 { x: 40.0, y: 40.0 }
            ]
        );
    }

    #[test]
    fn point_processing_works_correctly() {
        let dots = [
            Dot2D {
                position: Point2 { x: 584.5, y: 630.0 },
                radius: 2.0,
            },
            Dot2D {
                position: Point2 { x: 582.5, y: 644.0 },
                radius: 2.0,
            },
            Dot2D {
                position: Point2 { x: 581.5, y: 649.0 },
                radius: 2.0,
            },
            Dot2D {
                position: Point2 { x: 581.5, y: 651.0 },
                radius: 2.0,
            },
        ];

        let bezier = catmull_rom_to_bezier([dots[0], dots[1], dots[2], dots[3]]);

        let diff_start_end = dots[0].position.sub_element_wise(dots[3].position);
        let len_start_end = sqr_len(diff_start_end).sqrt();
        let dots_count = len_start_end * 2.;

        let result = eval_bezier(bezier, dots_count as usize);

        assert_eq!(result.len(), 42);
        let expected_data = vec![
            (582.465_7, 644.214_84, 2.0),
            (582.432, 644.421_94, 2.0),
            (582.398_74, 644.621_46, 2.0),
            (582.366_1, 644.813_8, 2.0),
            (582.333_9, 644.998_96, 2.0),
            (582.302_3, 645.177_3, 2.0),
            (582.271_2, 645.349, 2.0),
            (582.240_66, 645.514_34, 2.0),
            (582.210_63, 645.673_4, 2.0),
            (582.181_1, 645.826_54, 2.0),
            (582.152_1, 645.973_94, 2.0),
            (582.123_66, 646.115_7, 2.0),
            (582.095_76, 646.252_26, 2.0),
            (582.068_36, 646.383_67, 2.0),
            (582.041_56, 646.510_25, 2.0),
            (582.015_2, 646.632_14, 2.0),
            (581.989_44, 646.749_6, 2.0),
            (581.964_2, 646.862_8, 2.0),
            (581.939_45, 646.972_05, 2.0),
            (581.915_2, 647.077_45, 2.0),
            (581.891_54, 647.179_4, 2.0),
            (581.868_4, 647.277_95, 2.0),
            (581.845_8, 647.373_35, 2.0),
            (581.823_7, 647.465_94, 2.0),
            (581.802_2, 647.555_8, 2.0),
            (581.781_1, 647.643_2, 2.0),
            (581.760_7, 647.728_33, 2.0),
            (581.740_7, 647.811_5, 2.0),
            (581.721_25, 647.892_8, 2.0),
            (581.702_4, 647.972_6, 2.0),
            (581.684, 648.051, 2.0),
            (581.666_2, 648.128_3, 2.0),
            (581.648_86, 648.204_6, 2.0),
            (581.632_1, 648.280_3, 2.0),
            (581.615_84, 648.355_4, 2.0),
            (581.600_1, 648.430_3, 2.0),
            (581.584_96, 648.505_2, 2.0),
            (581.570_3, 648.580_2, 2.0),
            (581.556_15, 648.655_64, 2.0),
            (581.542_54, 648.731_7, 2.0),
            (581.529_5, 648.808_6, 2.0),
            (581.516_97, 648.886_54, 2.0),
        ];

        for (i, &(expected_x, expected_y, expected_r)) in expected_data.iter().enumerate() {
            let dot = &result[i];
            assert!(
                (dot.position.x - expected_x).abs() < 1.,
                "x mismatch at index {}: expected {}, got {}",
                i,
                expected_x,
                dot.position.x
            );
            assert!(
                (dot.position.y - expected_y).abs() < 1.,
                "y mismatch at index {}: expected {}, got {}",
                i,
                expected_y,
                dot.position.y
            );
            assert!(
                (dot.radius - expected_r).abs() < 1.,
                "radius mismatch at index {}: expected {}, got {}",
                i,
                expected_r,
                dot.radius
            );
        }
    }
}
