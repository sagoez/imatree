use image::{DynamicImage, ImageBuffer, Rgba};
use text_to_png::{Color as RendererColor, FontSize, TextPixmap, TextPng, TextRenderer};

use crate::{
    domain::{Background, CanvasSize, ImageSpec},
    error::RenderError,
    layout::{Layout, MeasureText, REFERENCE_SIZE, plan_layout, to_f64},
};

/// Glyphs used to establish the ascent and descent of a line of text.
const METRIC_SAMPLE: &str = "Hg";

/// Renders a valid image specification entirely in memory.
///
/// # Errors
///
/// Returns [`RenderError`] if the caption cannot be drawn, cannot be decoded,
/// or cannot be laid out legibly within the requested canvas.
pub fn render_image(spec: &ImageSpec) -> Result<DynamicImage, RenderError> {
    let renderer = CaptionRenderer::new()?;
    let canvas = spec.canvas();
    let layout = plan_layout(
        spec.caption(),
        canvas,
        spec.text_style().sizing(),
        &renderer,
    )?;
    let mut image = blank_canvas(canvas, spec.background());

    draw_lines(
        &mut image,
        &renderer,
        &layout,
        canvas,
        spec.text_style().color().value(),
    )?;

    Ok(match spec.background() {
        Background::Transparent => image,
        Background::Solid(_) => DynamicImage::ImageRgb8(image.to_rgb8()),
    })
}

fn blank_canvas(canvas: CanvasSize, background: Background) -> DynamicImage {
    let fill = match background {
        Background::Transparent => Rgba([0, 0, 0, 0]),
        Background::Solid(color) => {
            let (red, green, blue) = color.rgb();

            Rgba([red, green, blue, u8::MAX])
        }
    };

    DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
        canvas.width(),
        canvas.height(),
        fill,
    ))
}

/// Draws each wrapped line centred horizontally and aligned to its baseline.
fn draw_lines(
    image: &mut DynamicImage,
    renderer: &CaptionRenderer,
    layout: &Layout,
    canvas: CanvasSize,
    color: RendererColor,
) -> Result<(), RenderError> {
    for (index, line) in layout.lines.iter().enumerate() {
        let drawn = renderer.render(line, layout.font_size, color)?;
        let overlay = image::load_from_memory(&drawn.data).map_err(RenderError::Decode)?;
        let baseline = layout.first_baseline + to_f64(index) * layout.line_height;
        let left = (f64::from(canvas.width()) - f64::from(drawn.size.width)) / 2.0;

        image::imageops::overlay(
            image,
            &overlay,
            round(left),
            round(baseline - drawn.baseline_down_from_top),
        );
    }

    Ok(())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "positions are bounded by the canvas, and the saturating cast is exact enough for a pixel offset"
)]
fn round(value: f64) -> i64 {
    value.round() as i64
}

/// The text renderer, plus the vertical metrics of its default font.
struct CaptionRenderer<'a> {
    renderer: TextRenderer<'a>,
    ascent: f64,
    descent: f64,
}

impl CaptionRenderer<'_> {
    fn new() -> Result<Self, RenderError> {
        let renderer = TextRenderer::default();
        let sample = measure(&renderer, METRIC_SAMPLE)?;

        Ok(Self {
            renderer,
            ascent: sample.baseline_down_from_top,
            descent: f64::from(sample.size.height) - sample.baseline_down_from_top,
        })
    }

    fn render(
        &self,
        text: &str,
        font_size: f64,
        color: RendererColor,
    ) -> Result<TextPng, RenderError> {
        self.renderer
            .render_text_to_png_data(text, FontSize::Direct(font_size), color)
            .map_err(text_render_error)
    }
}

/// Rasterises `text` at [`REFERENCE_SIZE`] for its metrics alone, so it skips
/// the PNG encoding the drawing path needs. The colour does not affect metrics.
fn measure(renderer: &TextRenderer<'_>, text: &str) -> Result<TextPixmap, RenderError> {
    renderer
        .render_text_to_pixmap(text, FontSize::Direct(REFERENCE_SIZE), "Black")
        .map_err(text_render_error)
}

fn text_render_error(error: text_to_png::TextToPngError) -> RenderError {
    RenderError::TextRender(Box::new(error))
}

impl MeasureText for CaptionRenderer<'_> {
    fn width(&self, text: &str) -> Result<f64, RenderError> {
        Ok(f64::from(measure(&self.renderer, text)?.size.width))
    }

    fn ascent(&self) -> f64 {
        self.ascent
    }

    fn descent(&self) -> f64 {
        self.descent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Background, CanvasSize, Caption, Color, FontSizing, TextStyle};
    use image::{GenericImageView, Rgba};

    fn spec(text: &str, width: u32, height: u32, background: &str) -> ImageSpec {
        ImageSpec::new(
            Caption::new(text).unwrap(),
            CanvasSize::new(width, height).unwrap(),
            TextStyle::new(Color::parse("Black").unwrap(), FontSizing::Automatic),
            Background::parse(background).unwrap(),
        )
    }

    /// Bounding box of every pixel that differs from the canvas corner.
    fn ink_bounds(image: &DynamicImage) -> (u32, u32, u32, u32) {
        let pixels = image.to_rgba8();
        let background = *pixels.get_pixel(0, 0);
        let mut bounds: Option<(u32, u32, u32, u32)> = None;

        for (x, y, pixel) in pixels.enumerate_pixels() {
            if *pixel != background {
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((left, top, right, bottom)) => {
                        (left.min(x), top.min(y), right.max(x), bottom.max(y))
                    }
                });
            }
        }

        bounds.expect("expected the caption to leave some ink")
    }

    #[test]
    fn renders_the_requested_canvas_in_memory() {
        let image = render_image(&spec("Imatree", 320, 180, "White")).unwrap();

        assert_eq!(image.dimensions(), (320, 180));
    }

    #[test]
    fn long_captions_stay_inside_a_small_canvas() {
        let image = render_image(&spec(
            "A rather long caption that will not fit",
            200,
            100,
            "White",
        ))
        .unwrap();
        let (left, top, right, bottom) = ink_bounds(&image);

        assert!(left > 0 && top > 0, "text is clipped at the top left");
        assert!(
            right < 199 && bottom < 99,
            "text is clipped at the bottom right: {right}, {bottom}"
        );
    }

    #[test]
    fn solid_backgrounds_fill_the_canvas() {
        let image = render_image(&spec("Imatree", 100, 100, "#4a90e2")).unwrap();

        assert_eq!(
            image.to_rgba8().get_pixel(0, 0),
            &Rgba([0x4a, 0x90, 0xe2, 255])
        );
    }

    #[test]
    fn transparent_backgrounds_keep_an_alpha_channel() {
        let image = render_image(&spec("Imatree", 100, 100, "transparent")).unwrap();

        assert_eq!(image.to_rgba8().get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn captions_that_cannot_be_legible_are_rejected() {
        let result = render_image(&spec(
            "A rather long caption that will never fit here",
            16,
            16,
            "White",
        ));

        assert!(matches!(result, Err(RenderError::DoesNotFit { .. })));
    }
}
