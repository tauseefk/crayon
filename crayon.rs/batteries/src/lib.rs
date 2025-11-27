#![allow(dead_code)]

mod batteries;
mod math;
mod point_processor;
mod transformations;

pub mod prelude {
    pub use std::collections::VecDeque;
    pub use std::fmt;

    pub use cgmath::{Array, ElementWise, Point2, Vector2};

    pub use crate::batteries::*;
    pub use crate::math::*;
    pub use crate::point_processor::*;
    pub use crate::transformations::*;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn it_works() {
        let result = clamp(2., 1., 1.8);
        assert_eq!(result, 1.8);
    }
}
