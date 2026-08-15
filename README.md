# Imatree

[![crates.io](https://img.shields.io/crates/v/imatree.svg)](https://crates.io/crates/imatree)
[![docs.rs](https://docs.rs/imatree/badge.svg)](https://docs.rs/imatree)
[![CI](https://github.com/sagoez/imatree/actions/workflows/ci.yaml/badge.svg)](https://github.com/sagoez/imatree/actions/workflows/ci.yaml)

Generate a PNG of whatever string you want. The caption is wrapped and scaled to
fill the canvas, so it stays readable whatever size you ask for.

![Imatree](./sample/imatree.png)

## Install

```console
cargo install imatree
```

## Usage

```console
imatree --name "Functional domains" --color "#4a90e2"
```

Writes `functional_domains.png` in the current directory and prints the path.

A few more:

```console
# Wrapped and scaled to fit a banner
imatree --name "A rather long caption that will not fit" --width 1200 -H 300

# Transparent background, written where you want it
imatree --name "Overlay" --background transparent --output ./assets/overlay.png

# Pin the font size instead of fitting it to the canvas
imatree --name "Fixed" --font-size 96
```

## Options

```
  -n, --name <NAME>              The text to display in the image
  -p, --path <PATH>              Directory in which to save the image [default: .]
  -o, --output <OUTPUT>          File to write, instead of a name derived from the caption
  -c, --color <COLOR>            Text color as a name or RGB hex value [default: Black]
  -b, --background <BACKGROUND>  Background as a name, an RGB hex value, or "transparent" [default: White]
  -f, --font-size <FONT_SIZE>    Font size in pixels; omit to size the text to the canvas
  -w, --width <WIDTH>            Image width in pixels [default: 1000]
  -H, --height <HEIGHT>          Image height in pixels [default: 1000]
      --create-dirs              Create the output directory if it is missing
  -q, --quiet                    Do not print the path of the written image
  -h, --help                     Print help
  -V, --version                  Print version
```

With `--font-size` omitted, the caption is wrapped across as many lines as it
takes and drawn at the largest size that still fits the canvas. Passing an
explicit `--font-size` still wraps the caption, but never shrinks it, so a large
enough value will overflow the canvas.

The output file name is derived from the caption (lowercased, non-alphanumeric
runs collapsed to `_`) unless `--output` names a file. An existing file at that
path is overwritten.

## Library

```rust
use imatree::{Background, CanvasSize, Caption, Color, FontSizing, ImageSpec, TextStyle};

let spec = ImageSpec::new(
    Caption::new("Functional domains")?,
    CanvasSize::new(1000, 1000)?,
    TextStyle::new(Color::parse("#4a90e2")?, FontSizing::Automatic),
    Background::parse("White")?,
);

imatree::render_image(&spec)?.save("functional_domains.png")?;
```

Every input is validated on the way in: [`Caption`], [`CanvasSize`], [`Color`]
and [`FontSizing`] each return a `DomainError` rather than accepting something
unrenderable, and `render_image` returns a `RenderError` describing what went
wrong.

## License

MIT. See [LICENCE](./LICENCE).
