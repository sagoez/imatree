use crate::{
    domain::{CanvasSize, Caption, FontSizing},
    error::RenderError,
};

/// Size at which text is measured; every layout figure is scaled from here.
pub(crate) const REFERENCE_SIZE: f64 = 100.0;

/// Baseline-to-baseline distance as a multiple of the font size.
const LINE_SPACING: f64 = 1.25;

/// Fraction of each canvas dimension kept clear on both sides.
const MARGIN_RATIO: f64 = 0.05;

/// Smallest font size worth producing; below this the caption is unreadable.
const MIN_FONT_SIZE: f64 = 4.0;

/// Text measurements taken at [`REFERENCE_SIZE`].
pub(crate) trait MeasureText {
    /// Width of `text` rendered at [`REFERENCE_SIZE`].
    fn width(&self, text: &str) -> Result<f64, RenderError>;

    /// Distance from the top of the tallest glyph to the baseline.
    fn ascent(&self) -> f64;

    /// Distance from the baseline to the bottom of the deepest glyph.
    fn descent(&self) -> f64;
}

/// Where each line of a wrapped caption belongs on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layout {
    pub font_size: f64,
    pub lines: Vec<String>,
    pub line_height: f64,
    pub first_baseline: f64,
}

/// Wraps the caption and picks the largest font size that keeps it on the canvas.
///
/// [`FontSizing::Fixed`] is honoured as given: the caption still wraps, but it is
/// never shrunk to fit, so an oversized request can overflow the canvas.
pub(crate) fn plan_layout(
    caption: &Caption,
    canvas: CanvasSize,
    sizing: FontSizing,
    measurer: &impl MeasureText,
) -> Result<Layout, RenderError> {
    let words: Vec<&str> = caption.words().collect();
    let widths = words
        .iter()
        .map(|word| measurer.width(word))
        .collect::<Result<Vec<_>, _>>()?;
    let space = space_width(measurer)?;

    let available_width = f64::from(canvas.width()) * (1.0 - 2.0 * MARGIN_RATIO);
    let available_height = f64::from(canvas.height()) * (1.0 - 2.0 * MARGIN_RATIO);
    let automatic = sizing == FontSizing::Automatic;

    let mut font_size = match sizing {
        FontSizing::Fixed(pixels) => f64::from(pixels.get()),
        FontSizing::Automatic => {
            largest_fitting_size(&widths, space, available_width, available_height, measurer)
                .ok_or_else(|| RenderError::does_not_fit(canvas))?
        }
    };

    let lines: Vec<String> = wrap(&widths, space, reference_width(available_width, font_size))
        .into_iter()
        .map(|line| words[line].join(" "))
        .collect();

    // Word-by-word widths only approximate a rendered line, so the chosen size is
    // trimmed against the real thing before anything is drawn.
    if automatic {
        let widest = measure_widest(&lines, measurer)?;
        font_size = font_size.min(available_width * REFERENCE_SIZE / widest);

        if font_size < MIN_FONT_SIZE {
            return Err(RenderError::does_not_fit(canvas));
        }
    }

    let scale = font_size / REFERENCE_SIZE;
    let line_height = font_size * LINE_SPACING;
    let ascent = measurer.ascent() * scale;
    let block_height = ascent + measurer.descent() * scale + to_f64(lines.len() - 1) * line_height;

    Ok(Layout {
        font_size,
        lines,
        line_height,
        first_baseline: (f64::from(canvas.height()) - block_height) / 2.0 + ascent,
    })
}

/// Converts a small count or index to a float without tripping the precision lint.
pub(crate) fn to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

/// The width available to a line, expressed at [`REFERENCE_SIZE`].
fn reference_width(available_width: f64, font_size: f64) -> f64 {
    available_width * REFERENCE_SIZE / font_size
}

/// Width of a single space, inferred from a pair of glyphs measured with and
/// without one between them.
fn space_width(measurer: &impl MeasureText) -> Result<f64, RenderError> {
    Ok((measurer.width("n n")? - measurer.width("nn")?).max(0.0))
}

fn measure_widest(lines: &[String], measurer: &impl MeasureText) -> Result<f64, RenderError> {
    let mut widest = f64::MIN_POSITIVE;

    for line in lines {
        widest = widest.max(measurer.width(line)?);
    }

    Ok(widest)
}

/// Greedily groups words into lines no wider than `limit` at [`REFERENCE_SIZE`].
///
/// A word wider than `limit` on its own still gets a line of its own.
fn wrap(widths: &[f64], space: f64, limit: f64) -> Vec<std::ops::Range<usize>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut current = 0.0;

    for (index, &width) in widths.iter().enumerate() {
        if index == start {
            current = width;
        } else if current + space + width <= limit {
            current += space + width;
        } else {
            lines.push(start..index);
            start = index;
            current = width;
        }
    }

    lines.push(start..widths.len());
    lines
}

/// Binary searches for the largest font size whose wrapped block fits.
///
/// Taller text needs more lines, so fitting is monotonic in the font size.
fn largest_fitting_size(
    widths: &[f64],
    space: f64,
    available_width: f64,
    available_height: f64,
    measurer: &impl MeasureText,
) -> Option<f64> {
    let fits = |size: f64| {
        let limit = reference_width(available_width, size);
        let lines = wrap(widths, space, limit);
        let widest = lines
            .iter()
            .map(|line| line_width(widths, space, line.clone()))
            .fold(0.0_f64, f64::max);
        let scale = size / REFERENCE_SIZE;
        let height = (measurer.ascent() + measurer.descent()) * scale
            + to_f64(lines.len() - 1) * size * LINE_SPACING;

        widest <= limit && height <= available_height
    };

    if !fits(MIN_FONT_SIZE) {
        return None;
    }

    let mut low = MIN_FONT_SIZE;
    let mut high = available_height.max(MIN_FONT_SIZE);

    if fits(high) {
        return Some(high);
    }

    for _ in 0..30 {
        let middle = f64::midpoint(low, high);

        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }

    Some(low)
}

fn line_width(widths: &[f64], space: f64, line: std::ops::Range<usize>) -> f64 {
    let words = to_f64(line.len().saturating_sub(1));

    widths[line].iter().sum::<f64>() + words * space
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monospace stand-in: every glyph is a tenth of the reference size wide.
    struct Monospace;

    impl MeasureText for Monospace {
        fn width(&self, text: &str) -> Result<f64, RenderError> {
            Ok(to_f64(text.chars().count()) * REFERENCE_SIZE / 10.0)
        }

        fn ascent(&self) -> f64 {
            REFERENCE_SIZE * 0.8
        }

        fn descent(&self) -> f64 {
            REFERENCE_SIZE * 0.2
        }
    }

    fn plan(text: &str, width: u32, height: u32, sizing: FontSizing) -> Layout {
        plan_layout(
            &Caption::new(text).unwrap(),
            CanvasSize::new(width, height).unwrap(),
            sizing,
            &Monospace,
        )
        .expect("caption should fit")
    }

    /// Widest line and total block height, in pixels, as the fake font would draw them.
    fn block_size(layout: &Layout) -> (f64, f64) {
        let scale = layout.font_size / REFERENCE_SIZE;
        let widest = layout
            .lines
            .iter()
            .map(|line| Monospace.width(line).unwrap() * scale)
            .fold(0.0_f64, f64::max);
        let height = (Monospace.ascent() + Monospace.descent()) * scale
            + to_f64(layout.lines.len() - 1) * layout.line_height;

        (widest, height)
    }

    #[test]
    fn short_captions_stay_on_one_line() {
        let layout = plan("Imatree", 1000, 1000, FontSizing::Automatic);

        assert_eq!(layout.lines, vec!["Imatree"]);
    }

    #[test]
    fn long_captions_wrap_instead_of_shrinking_to_nothing() {
        let layout = plan(
            "A rather long caption that will not fit on a single line",
            1000,
            1000,
            FontSizing::Automatic,
        );

        assert!(layout.lines.len() > 1, "expected wrapping: {layout:?}");
        assert!(
            layout.font_size > 40.0,
            "wrapping should keep the text large: {layout:?}"
        );
    }

    #[test]
    fn automatic_sizing_keeps_the_text_inside_a_small_canvas() {
        let layout = plan(
            "A rather long caption that will not fit",
            200,
            100,
            FontSizing::Automatic,
        );
        let (width, height) = block_size(&layout);

        assert!(width <= 200.0, "text is {width} wide on a 200px canvas");
        assert!(height <= 100.0, "text is {height} tall on a 100px canvas");
    }

    #[test]
    fn automatic_sizing_uses_the_space_available() {
        let layout = plan("Hi", 1000, 1000, FontSizing::Automatic);
        let (width, height) = block_size(&layout);

        assert!(
            width > 500.0 || height > 500.0,
            "text should grow to fill the canvas: {layout:?}"
        );
    }

    #[test]
    fn words_are_never_split_across_lines() {
        let layout = plan(
            "alpha bravo charlie delta echo foxtrot",
            300,
            300,
            FontSizing::Automatic,
        );

        assert_eq!(
            layout.lines.join(" "),
            "alpha bravo charlie delta echo foxtrot"
        );
    }

    #[test]
    fn whitespace_in_the_caption_is_collapsed() {
        let layout = plan("  spaced \n\t out  ", 1000, 1000, FontSizing::Automatic);

        assert_eq!(layout.lines, vec!["spaced out"]);
    }

    #[test]
    fn fixed_sizing_is_honoured_exactly() {
        let layout = plan("Imatree", 1000, 1000, FontSizing::fixed(50).unwrap());

        assert!((layout.font_size - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fixed_sizing_still_wraps_to_the_canvas() {
        let layout = plan(
            "alpha bravo charlie delta",
            100,
            1000,
            FontSizing::fixed(50).unwrap(),
        );

        assert!(layout.lines.len() > 1, "expected wrapping: {layout:?}");
    }

    #[test]
    fn captions_that_cannot_be_legible_are_rejected() {
        let result = plan_layout(
            &Caption::new("A rather long caption that will never fit here").unwrap(),
            CanvasSize::new(10, 10).unwrap(),
            FontSizing::Automatic,
            &Monospace,
        );

        assert!(matches!(
            result,
            Err(RenderError::DoesNotFit {
                width: 10,
                height: 10
            })
        ));
    }

    #[test]
    fn the_text_block_is_centred_vertically() {
        let layout = plan("Imatree", 1000, 400, FontSizing::Automatic);
        let scale = layout.font_size / REFERENCE_SIZE;
        let block_height = (Monospace.ascent() + Monospace.descent()) * scale;
        let expected = (400.0 - block_height) / 2.0 + Monospace.ascent() * scale;

        assert!(
            (layout.first_baseline - expected).abs() < 0.001,
            "baseline {} should be {expected}",
            layout.first_baseline
        );
    }
}
