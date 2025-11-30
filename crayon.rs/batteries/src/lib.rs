#![warn(clippy::pedantic)]

mod batteries;
mod math;
mod point_processor;
mod transformations;

pub mod prelude {
    pub use core::fmt;
    pub use std::collections::VecDeque;

    pub use cgmath::{Array, ElementWise, Point2, Vector2};

    pub use crate::batteries::*;
    pub use crate::math::*;
    pub use crate::point_processor::*;
    pub use crate::transformations::*;
}
