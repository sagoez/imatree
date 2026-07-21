use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use imatree::{render_image, CanvasSize, Caption, FontSizing, ImageSpec, TextColor, TextStyle};

/// Creates a PNG with centered text on a white background.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// The text to display in the image
    #[arg(short, long)]
    name: String,

    /// Directory in which to save the image
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Text color as a name or RGB hex value
    #[arg(short, long, default_value = "Black")]
    color: String,

    /// Font size in pixels; zero selects automatic sizing
    #[arg(short, long, default_value_t = 0)]
    font_size: u32,

    /// Image width in pixels
    #[arg(short, long, default_value_t = 1000)]
    width: u32,

    /// Image height in pixels
    #[arg(short = 't', long, default_value_t = 1000)]
    height: u32,
}

fn main() -> Result<()> {
    run(Args::parse())
}

fn run(args: Args) -> Result<()> {
    let caption = Caption::new(args.name).context("invalid caption")?;
    let output_path = args.path.join(caption.output_file_name().as_str());
    let spec = ImageSpec::new(
        caption,
        CanvasSize::new(args.width, args.height).context("invalid canvas")?,
        TextStyle::new(
            TextColor::parse(&args.color).context("invalid text style")?,
            FontSizing::from_pixels(args.font_size),
        ),
    );

    render_image(&spec)?
        .save(&output_path)
        .with_context(|| format!("failed to save image to {}", output_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_defaults() {
        let args = Args::try_parse_from(["imatree", "--name", "Hello"]).unwrap();

        assert_eq!(args.path, PathBuf::from("."));
        assert_eq!(args.color, "Black");
        assert_eq!(args.font_size, 0);
        assert_eq!((args.width, args.height), (1000, 1000));
    }
}
