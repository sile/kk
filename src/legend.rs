//! Paints the key-binding legend.

use crate::terminal::{put_str, str_cols};

use crate::binding::{self, BORDER_HORIZONTAL, Context, LegendSize};

/// Paints the key-binding legend.
///
/// The legend sits in the frame's top-right corner: one binding per row, and
/// under them a horizontal border with the context title centered in it. Each
/// row draws its own left border, and no row draws a right border. It paints
/// nothing when the frame cannot hold the legend whole: a legend clipped to fit
/// would show chords without their labels.
#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    /// Paints the legend for `context` into the frame's top-right corner.
    pub fn render(&self, context: Context, frame: &mut tuinix::Frame) {
        let legend = binding::legend_size(context, frame.size());
        if legend != full_size(context) {
            return;
        }

        let style = tuinix::Style::new().reverse();
        let origin = tuinix::Position {
            row: 0,
            col: frame.size().cols - legend.cols,
        };

        for (i, row) in binding::legend(context).iter().enumerate() {
            let at = tuinix::Position {
                row: origin.row + i,
                col: origin.col,
            };
            put_str(frame, at, row, style);
        }

        let bottom = tuinix::Position {
            row: origin.row + binding::legend(context).len(),
            col: origin.col,
        };
        let border = bottom_border(binding::title(context), legend.cols);
        put_str(frame, bottom, &border, style);
    }
}

/// The size the legend of `context` needs, with room to spare.
fn full_size(context: Context) -> LegendSize {
    binding::legend_size(
        context,
        tuinix::Size {
            rows: usize::MAX,
            cols: usize::MAX,
        },
    )
}

/// Centers `title` between dashes, in a run of `width` columns.
///
/// The run is [`BORDER_HORIZONTAL`] repeated with ` title ` in the middle; a
/// title too wide for the run is dropped and the whole run is dashes.
fn bottom_border(title: &str, width: usize) -> String {
    let title_cols = str_cols(title);
    if title_cols == 0 || title_cols + 2 > width {
        return BORDER_HORIZONTAL.repeat(width);
    }
    let left = (width - title_cols - 2) / 2;
    let right = width - title_cols - 2 - left;
    format!(
        "{} {} {}",
        BORDER_HORIZONTAL.repeat(left),
        title,
        BORDER_HORIZONTAL.repeat(right)
    )
}
