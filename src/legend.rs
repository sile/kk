//! Paints the key-binding legend.

use crate::terminal::{put_str, str_cols};

use crate::binding::{self, Context, LegendSize};

/// Paints the key-binding legend.
///
/// The legend is a bordered box in the frame's top-right corner with one binding
/// per row and the context title centered in the bottom border. It paints
/// nothing when the frame cannot hold the box whole: a legend clipped to fit
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
        let right = origin.col + legend.cols - 1;

        for (i, row) in binding::legend(context).iter().enumerate() {
            let at = tuinix::Position {
                row: origin.row + i,
                col: origin.col,
            };
            let at = put_str(frame, at, "│", style);
            let at = put_str(frame, at, row, style);
            let pad = legend.cols - 2 - str_cols(row);
            let at = put_str(frame, at, &" ".repeat(pad), style);
            put_str(frame, at, "│", style);
        }

        let bottom = origin.row + binding::legend(context).len();
        let border = bottom_border(binding::title(context), legend.cols - 1);
        put_str(
            frame,
            tuinix::Position {
                row: bottom,
                col: origin.col,
            },
            &border,
            style,
        );
        put_str(
            frame,
            tuinix::Position {
                row: bottom,
                col: right,
            },
            "│",
            style,
        );
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
/// The run is `─` repeated with ` title ` in the middle; a title too wide for the
/// run is dropped and the whole run is dashes.
fn bottom_border(title: &str, width: usize) -> String {
    let title_cols = str_cols(title);
    if title_cols == 0 || title_cols + 2 > width {
        return "─".repeat(width);
    }
    let left = (width - title_cols - 2) / 2;
    let right = width - title_cols - 2 - left;
    format!("{} {} {}", "─".repeat(left), title, "─".repeat(right))
}
