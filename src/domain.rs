use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    num::NonZeroU32,
};

use text_to_png::Color;

/// A domain validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    EmptyCaption,
    ZeroWidth,
    ZeroHeight,
    InvalidTextColor(String),
}

impl Display for DomainError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCaption => formatter.write_str("caption must contain text"),
            Self::ZeroWidth => formatter.write_str("canvas width must be greater than zero"),
            Self::ZeroHeight => formatter.write_str("canvas height must be greater than zero"),
            Self::InvalidTextColor(color) => {
                write!(
                    formatter,
                    "'{color}' is not a valid color name or RGB hex value"
                )
            }
        }
    }
}

impl Error for DomainError {}

/// The text displayed in the generated image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption(String);

impl Caption {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();

        if value.trim().is_empty() {
            Err(DomainError::EmptyCaption)
        } else {
            Ok(Self(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn output_file_name(&self) -> OutputFileName {
        OutputFileName::from_caption(self)
    }

    fn character_count(&self) -> usize {
        self.0.chars().count()
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
    pub fn new(width: u32, height: u32) -> Result<Self, DomainError> {
        let width = NonZeroU32::new(width).ok_or(DomainError::ZeroWidth)?;
        let height = NonZeroU32::new(height).ok_or(DomainError::ZeroHeight)?;

        Ok(Self { width, height })
    }

    pub fn width(self) -> u32 {
        self.width.get()
    }

    pub fn height(self) -> u32 {
        self.height.get()
    }
}

/// The requested font-sizing rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSizing {
    Automatic,
    Fixed(NonZeroU32),
}

impl FontSizing {
    pub fn from_pixels(pixels: u32) -> Self {
        NonZeroU32::new(pixels).map_or(Self::Automatic, Self::Fixed)
    }

    pub fn pixels_for(self, caption: &Caption) -> u32 {
        match self {
            Self::Fixed(pixels) => pixels.get(),
            Self::Automatic => match caption.character_count() {
                0..=10 => 100,
                11..=20 => 75,
                21..=30 => 50,
                31..=40 => 25,
                41..=50 => 17,
                51..=60 => 10,
                _ => 5,
            },
        }
    }
}

/// A color accepted by the text renderer, parsed at the domain boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColor(Color);

impl TextColor {
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        Color::try_from(value)
            .map(Self)
            .map_err(|()| DomainError::InvalidTextColor(value.to_owned()))
    }

    pub(crate) fn value(self) -> Color {
        self.0
    }
}

/// The visual rules for rendering a caption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    color: TextColor,
    sizing: FontSizing,
}

impl TextStyle {
    pub fn new(color: TextColor, sizing: FontSizing) -> Self {
        Self { color, sizing }
    }

    pub(crate) fn color(self) -> Color {
        self.color.value()
    }

    pub(crate) fn font_size_for(self, caption: &Caption) -> u32 {
        self.sizing.pixels_for(caption)
    }
}

/// A complete, valid description of an image to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSpec {
    caption: Caption,
    canvas: CanvasSize,
    text_style: TextStyle,
}

impl ImageSpec {
    pub fn new(caption: Caption, canvas: CanvasSize, text_style: TextStyle) -> Self {
        Self {
            caption,
            canvas,
            text_style,
        }
    }

    pub fn caption(&self) -> &Caption {
        &self.caption
    }

    pub fn canvas(&self) -> CanvasSize {
        self.canvas
    }

    pub fn text_style(&self) -> TextStyle {
        self.text_style
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Position {
    pub x: i64,
    pub y: i64,
}

pub(crate) fn centered_position(
    canvas: CanvasSize,
    content_width: u32,
    content_height: u32,
) -> Position {
    Position {
        x: (i64::from(canvas.width()) - i64::from(content_width)).div_euclid(2),
        y: (i64::from(canvas.height()) - i64::from(content_height)).div_euclid(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caption_with_length(length: usize) -> Caption {
        Caption::new("x".repeat(length)).expect("test caption is non-empty")
    }

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
    fn automatic_font_size_uses_character_count() {
        let sizing = FontSizing::Automatic;

        assert_eq!(sizing.pixels_for(&caption_with_length(10)), 100);
        assert_eq!(sizing.pixels_for(&caption_with_length(11)), 75);
        assert_eq!(
            sizing.pixels_for(&Caption::new("é".repeat(10)).unwrap()),
            100
        );
    }

    #[test]
    fn zero_font_size_means_automatic() {
        assert_eq!(FontSizing::from_pixels(0), FontSizing::Automatic);
        assert_eq!(
            FontSizing::from_pixels(42).pixels_for(&caption_with_length(1)),
            42
        );
    }

    #[test]
    fn invalid_colors_are_rejected_at_the_boundary() {
        assert_eq!(
            TextColor::parse("not a color"),
            Err(DomainError::InvalidTextColor("not a color".to_owned()))
        );
        assert!(TextColor::parse("#4a90e2").is_ok());
        assert!(TextColor::parse("Black").is_ok());
    }

    #[test]
    fn oversized_content_is_centered_without_unsigned_underflow() {
        let canvas = CanvasSize::new(100, 80).unwrap();

        assert_eq!(
            centered_position(canvas, 120, 100),
            Position { x: -10, y: -10 }
        );
    }
}
