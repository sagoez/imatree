//! Command line front end for the [`imatree`] library.

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use imatree::{
    Background, CanvasSize, Caption, Color, FontSizing, ImageSpec, TextStyle, render_image,
};

/// Creates a PNG with centered text, wrapped and sized to fit the canvas.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// The text to display in the image
    #[arg(short, long)]
    name: String,

    /// Directory in which to save the image
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// File to write, instead of a name derived from the caption
    #[arg(short, long, conflicts_with = "path")]
    output: Option<PathBuf>,

    /// Text color as a name or RGB hex value
    #[arg(short, long, default_value = "Black")]
    color: String,

    /// Background as a name, an RGB hex value, or "transparent"
    #[arg(short, long, default_value = "White")]
    background: String,

    /// Font size in pixels; omit to size the text to the canvas
    #[arg(short, long)]
    font_size: Option<u32>,

    /// Image width in pixels
    #[arg(short, long, default_value_t = 1000)]
    width: u32,

    /// Image height in pixels
    #[arg(short = 'H', long, default_value_t = 1000)]
    height: u32,

    /// Create the output directory if it is missing
    #[arg(long)]
    create_dirs: bool,

    /// Do not print the path of the written image
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> ExitCode {
    match run(&Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("imatree: {error:#}");

            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<()> {
    let caption = Caption::new(args.name.as_str())?;
    let sizing = match args.font_size {
        Some(pixels) => FontSizing::fixed(pixels)?,
        None => FontSizing::Automatic,
    };
    let spec = ImageSpec::new(
        caption,
        CanvasSize::new(args.width, args.height)?,
        TextStyle::new(Color::parse(&args.color)?, sizing),
        Background::parse(&args.background)?,
    );

    let target = args
        .output
        .clone()
        .unwrap_or_else(|| args.path.join(spec.caption().output_file_name().as_str()));
    ensure_directory(&target, args.create_dirs)?;

    render_image(&spec)?
        .save(&target)
        .with_context(|| format!("failed to save image to {}", target.display()))?;

    if !args.quiet {
        println!("Saved {}", target.display());
    }

    Ok(())
}

/// Checks the directory holding `target` before any rendering work is done.
fn ensure_directory(target: &Path, create: bool) -> Result<()> {
    let directory = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if directory.is_dir() {
        return Ok(());
    }

    if !create {
        bail!(
            "{} does not exist; pass --create-dirs to create it",
            directory.display()
        );
    }

    fs::create_dir_all(directory)
        .with_context(|| format!("failed to create {}", directory.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_defaults() {
        let args = Args::try_parse_from(["imatree", "--name", "Hello"]).unwrap();

        assert_eq!(args.path, PathBuf::from("."));
        assert_eq!(args.color, "Black");
        assert_eq!(args.background, "White");
        assert_eq!(args.font_size, None);
        assert_eq!((args.width, args.height), (1000, 1000));
        assert!(!args.create_dirs);
        assert!(!args.quiet);
    }

    #[test]
    fn an_explicit_output_cannot_be_combined_with_a_directory() {
        let result = Args::try_parse_from([
            "imatree",
            "--name",
            "Hello",
            "--path",
            "/tmp",
            "--output",
            "/tmp/x.png",
        ]);

        assert!(result.is_err());
    }
}
