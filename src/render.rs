use anyhow::{Context, Result};
use image::{DynamicImage, ImageBuffer, Rgb};
use text_to_png::TextRenderer;

use crate::{domain::centered_position, ImageSpec};

/// Renders a valid image specification entirely in memory.
pub fn render_image(spec: &ImageSpec) -> Result<DynamicImage> {
    let canvas = spec.canvas();
    let style = spec.text_style();
    let caption = spec.caption();

    let rendered_text = TextRenderer::default()
        .render_text_to_png_data(
            caption.as_str(),
            style.font_size_for(caption),
            style.color(),
        )
        .context("failed to render caption")?;

    let overlay = image::load_from_memory(&rendered_text.data)
        .context("failed to decode rendered caption")?;
    let mut image = DynamicImage::ImageRgb8(ImageBuffer::from_pixel(
        canvas.width(),
        canvas.height(),
        Rgb([255, 255, 255]),
    ));
    let position = centered_position(canvas, overlay.width(), overlay.height());

    image::imageops::overlay(&mut image, &overlay, position.x, position.y);

    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanvasSize, Caption, FontSizing, TextColor, TextStyle};
    use image::GenericImageView;

    #[test]
    fn renders_the_requested_canvas_in_memory() {
        let spec = ImageSpec::new(
            Caption::new("Imatree").unwrap(),
            CanvasSize::new(320, 180).unwrap(),
            TextStyle::new(TextColor::parse("Black").unwrap(), FontSizing::Automatic),
        );

        let image = render_image(&spec).unwrap();

        assert_eq!(image.dimensions(), (320, 180));
    }
}
