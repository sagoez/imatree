use std::error::Error;

use crate::domain::CanvasSize;

/// A failure encountered while turning a valid [`ImageSpec`] into pixels.
///
/// [`ImageSpec`]: crate::ImageSpec
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The text renderer could not draw the caption.
    #[error("failed to render the caption text")]
    TextRender(#[source] Box<dyn Error + Send + Sync>),

    /// The renderer produced a PNG that could not be decoded.
    #[error("failed to decode the rendered caption")]
    Decode(#[source] image::ImageError),

    /// The caption cannot be laid out legibly within the requested canvas.
    #[error(
        "caption does not fit legibly in a {width}x{height} canvas; \
         use a larger canvas or a shorter caption"
    )]
    DoesNotFit {
        /// Canvas width in pixels.
        width: u32,
        /// Canvas height in pixels.
        height: u32,
    },
}

impl RenderError {
    pub(crate) fn does_not_fit(canvas: CanvasSize) -> Self {
        Self::DoesNotFit {
            width: canvas.width(),
            height: canvas.height(),
        }
    }
}
