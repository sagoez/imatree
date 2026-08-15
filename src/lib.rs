//! Domain model and rendering boundary for Imatree.
//!
//! A caption, a canvas and a style are validated into an [`ImageSpec`], which
//! [`render_image`] turns into an in-memory image.
//!
//! ```
//! use imatree::{Background, CanvasSize, Caption, Color, FontSizing, ImageSpec, TextStyle};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let spec = ImageSpec::new(
//!     Caption::new("Functional domains")?,
//!     CanvasSize::new(1000, 1000)?,
//!     TextStyle::new(Color::parse("#4a90e2")?, FontSizing::Automatic),
//!     Background::parse("White")?,
//! );
//!
//! let image = imatree::render_image(&spec)?;
//! assert_eq!(image.width(), 1000);
//! # Ok(())
//! # }
//! ```

mod domain;
mod error;
mod layout;
mod render;

pub use domain::{
    Background, CanvasSize, Caption, Color, DomainError, FontSizing, ImageSpec, OutputFileName,
    TextStyle,
};
pub use error::RenderError;
pub use render::render_image;
