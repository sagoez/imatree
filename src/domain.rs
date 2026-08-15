use std::{
    fmt::{self, Display, Formatter},
    num::NonZeroU32,
};

use text_to_png::Color as RendererColor;

/// A domain validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DomainError {
    /// The caption held no printable text.
    #[error("caption must contain text")]
    EmptyCaption,

    /// The requested canvas width was zero.
    #[error("canvas width must be greater than zero")]
    ZeroWidth,

    /// The requested canvas height was zero.
    #[error("canvas height must be greater than zero")]
    ZeroHeight,

    /// An explicit font size of zero was requested.
    #[error("font size must be greater than zero")]
    ZeroFontSize,

    /// The colour was neither a known name nor an RGB hex value.
    #[error("'{0}' is not a valid color name or RGB hex value")]
    InvalidColor(String),
}

/// The text displayed in the generated image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption(String);

impl Caption {
    /// Creates a caption, rejecting text that is empty or entirely whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::EmptyCaption`] if `value` has no printable text.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();

        if value.trim().is_empty() {
            Err(DomainError::EmptyCaption)
        } else {
            Ok(Self(value))
        }
    }

    /// The caption text as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The caption split into words, with runs of whitespace collapsed.
    pub(crate) fn words(&self) -> impl Iterator<Item = &str> {
        self.0.split_whitespace()
    }

    /// A PNG file name derived deterministically from this caption.
    #[must_use]
    pub fn output_file_name(&self) -> OutputFileName {
        OutputFileName::from_caption(self)
    }
}

impl Display for Caption {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A safe PNG file name derived deterministically from a caption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFileName(String);

impl OutputFileName {
    fn from_caption(caption: &Caption) -> Self {
        let mut stem = String::new();
        let mut previous_was_separator = false;

        for character in caption.as_str().trim().chars().flat_map(char::to_lowercase) {
            if character.is_alphanumeric() {
                stem.push(character);
                previous_was_separator = false;
            } else if (character.is_whitespace() || character == '_')
                && !stem.is_empty()
                && !previous_was_separator
            {
                stem.push('_');
                previous_was_separator = true;
            }
        }

        while stem.ends_with('_') {
            stem.pop();
        }

        if stem.is_empty() {
            stem.push_str("image");
        }

        stem.push_str(".png");
        Self(stem)
    }

    /// The file name, including the `.png` extension.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for OutputFileName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Non-zero canvas dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

impl CanvasSize {
    /// Creates a canvas size from pixel dimensions.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ZeroWidth`] or [`DomainError::ZeroHeight`] if
    /// either dimension is zero.
    pub fn new(width: u32, height: u32) -> Result<Self, DomainError> {
        let width = NonZeroU32::new(width).ok_or(DomainError::ZeroWidth)?;
        let height = NonZeroU32::new(height).ok_or(DomainError::ZeroHeight)?;

        Ok(Self { width, height })
    }

    /// Canvas width in pixels.
    #[must_use]
    pub fn width(self) -> u32 {
        self.width.get()
    }

    /// Canvas height in pixels.
    #[must_use]
    pub fn height(self) -> u32 {
        self.height.get()
    }
}

/// The requested font-sizing rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSizing {
    /// Grow the caption to the largest size that fits the canvas.
    Automatic,

    /// Draw the caption at exactly this many pixels.
    Fixed(NonZeroU32),
}

impl FontSizing {
    /// Requests an exact font size in pixels.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ZeroFontSize`] if `pixels` is zero.
    pub fn fixed(pixels: u32) -> Result<Self, DomainError> {
        NonZeroU32::new(pixels)
            .map(Self::Fixed)
            .ok_or(DomainError::ZeroFontSize)
    }
}

/// A colour accepted by the renderer, parsed at the domain boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(RendererColor);

impl Color {
    /// Parses a colour name (`"Black"`) or RGB hex value (`"#4a90e2"`).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidColor`] if `value` is neither.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        RendererColor::try_from(value)
            .map(Self)
            .map_err(|()| DomainError::InvalidColor(value.to_owned()))
    }

    /// The red, green and blue channels.
    #[must_use]
    pub fn rgb(self) -> (u8, u8, u8) {
        (self.0.r, self.0.g, self.0.b)
    }

    pub(crate) fn value(self) -> RendererColor {
        self.0
    }
}

/// What the caption is drawn onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    /// Leave the canvas fully transparent.
    Transparent,

    /// Fill the canvas with a single colour.
    Solid(Color),
}

impl Background {
    /// Parses `"transparent"` or any colour accepted by [`Color::parse`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidColor`] if `value` is neither.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        if value.trim().eq_ignore_ascii_case("transparent") {
            Ok(Self::Transparent)
        } else {
            Color::parse(value).map(Self::Solid)
        }
    }
}

/// The visual rules for rendering a caption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    color: Color,
    sizing: FontSizing,
}

impl TextStyle {
    /// Combines a text colour with a font-sizing rule.
    #[must_use]
    pub fn new(color: Color, sizing: FontSizing) -> Self {
        Self { color, sizing }
    }

    /// The text colour.
    #[must_use]
    pub fn color(self) -> Color {
        self.color
    }

    /// The font-sizing rule.
    #[must_use]
    pub fn sizing(self) -> FontSizing {
        self.sizing
    }
}

/// A complete, valid description of an image to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSpec {
    caption: Caption,
    canvas: CanvasSize,
    text_style: TextStyle,
    background: Background,
}

impl ImageSpec {
    /// Assembles a specification from already-validated parts.
    #[must_use]
    pub fn new(
        caption: Caption,
        canvas: CanvasSize,
        text_style: TextStyle,
        background: Background,
    ) -> Self {
        Self {
            caption,
            canvas,
            text_style,
            background,
        }
    }

    /// The caption to draw.
    #[must_use]
    pub fn caption(&self) -> &Caption {
        &self.caption
    }

    /// The canvas dimensions.
    #[must_use]
    pub fn canvas(&self) -> CanvasSize {
        self.canvas
    }

    /// The text colour and sizing rule.
    #[must_use]
    pub fn text_style(&self) -> TextStyle {
        self.text_style
    }

    /// The canvas fill.
    #[must_use]
    pub fn background(&self) -> Background {
        self.background
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_rejects_blank_text() {
        assert_eq!(Caption::new(" \n\t"), Err(DomainError::EmptyCaption));
    }

    #[test]
    fn output_file_name_is_safe_and_stable() {
        let caption = Caption::new("  Hello,   Functional_World!  ").unwrap();

        assert_eq!(
            caption.output_file_name().as_str(),
            "hello_functional_world.png"
        );
    }

    #[test]
    fn output_file_name_has_a_fallback_for_symbols() {
        let caption = Caption::new("✨ !!!").unwrap();

        assert_eq!(caption.output_file_name().as_str(), "image.png");
    }

    #[test]
    fn canvas_dimensions_must_be_non_zero() {
        assert_eq!(CanvasSize::new(0, 100), Err(DomainError::ZeroWidth));
        assert_eq!(CanvasSize::new(100, 0), Err(DomainError::ZeroHeight));
    }

    #[test]
    fn an_explicit_font_size_must_be_non_zero() {
        assert_eq!(FontSizing::fixed(0), Err(DomainError::ZeroFontSize));
        assert!(matches!(FontSizing::fixed(42), Ok(FontSizing::Fixed(_))));
    }

    #[test]
    fn invalid_colors_are_rejected_at_the_boundary() {
        assert_eq!(
            Color::parse("not a color"),
            Err(DomainError::InvalidColor("not a color".to_owned()))
        );
        assert!(Color::parse("#4a90e2").is_ok());
        assert!(Color::parse("Black").is_ok());
    }

    #[test]
    fn colors_expose_their_channels() {
        assert_eq!(Color::parse("#4a90e2").unwrap().rgb(), (0x4a, 0x90, 0xe2));
    }

    #[test]
    fn backgrounds_accept_transparency_and_colors() {
        assert_eq!(
            Background::parse("transparent"),
            Ok(Background::Transparent)
        );
        assert_eq!(
            Background::parse("TRANSPARENT"),
            Ok(Background::Transparent)
        );
        assert_eq!(
            Background::parse("White"),
            Ok(Background::Solid(Color::parse("White").unwrap()))
        );
        assert_eq!(
            Background::parse("nonsense"),
            Err(DomainError::InvalidColor("nonsense".to_owned()))
        );
    }

    #[test]
    fn captions_are_split_into_words() {
        let caption = Caption::new("  two \n\t words  ").unwrap();

        assert_eq!(caption.words().collect::<Vec<_>>(), vec!["two", "words"]);
    }
}
