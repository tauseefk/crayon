use std::vec;

use crate::prelude::*;

const POINT_PROCESSOR_SIZE: usize = 4;

pub struct PointProcessor {
    dots: VecDeque<StrokeDot2D>,
    size: f32,
    step: f32,
    sharpness: f32,
    filter: DistanceFilter,
}

impl PointProcessor {
    pub fn new(size: f32, step: f32, sharpness: f32) -> Self {
        Self {
            dots: VecDeque::with_capacity(POINT_PROCESSOR_SIZE),
            size,
            step,
            sharpness,
            filter: DistanceFilter::new(step),
        }
    }

    pub fn process_point(&mut self, point: StrokeDot2D) -> Vec<Point2<f32>> {
        if self.dots.len() < POINT_PROCESSOR_SIZE {
            self.dots.push_back(point);
            return vec![];
        }

        let new_point = point;

        let diff_from_last_point = self.dots[3].position.sub_element_wise(new_point.position);
        let diff_square_len = sqr_len(diff_from_last_point);

        let mut out_dots: Vec<Point2<f32>> = vec![];
        if new_point.is_last || diff_square_len > self.step.powi(2) {
            let repeat: usize = match new_point.is_last {
                true => 3,
                false => 1,
            };

            for _ in 0..repeat {
                // pop first to prevent overflow
                self.dots.pop_front();
                self.dots.push_back(new_point);

                let bezier = catmull_rom_to_bezier([
                    self.dots[0].into(),
                    self.dots[1].into(),
                    self.dots[2].into(),
                    self.dots[3].into(),
                ]);

                let diff_start_end = self.dots[0]
                    .position
                    .sub_element_wise(self.dots[3].position);
                let len_start_end = sqr_len(diff_start_end).sqrt();
                let dots_count = len_start_end * 2.;

                let dots = eval_bezier(bezier, dots_count.floor() as usize);

                let dots: Vec<Point2<f32>> = dots
                    .into_iter()
                    .filter_map(|point| self.filter.filter_by_distance(point.position))
                    .collect();
                out_dots.extend(dots);
            }
        }

        out_dots
    }

    pub fn clear(&mut self) {
        self.dots.clear();
    }
}
