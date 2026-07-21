//! Domain model and rendering boundary for Imatree.

mod domain;
mod render;

pub use domain::{
    CanvasSize, Caption, DomainError, FontSizing, ImageSpec, OutputFileName, TextColor, TextStyle,
};
pub use render::render_image;
