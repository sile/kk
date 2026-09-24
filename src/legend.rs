//! Paints the key-binding legend.

use crate::terminal::put_str;

use crate::binding::{self, Context, LegendSize};

/// Paints the key-binding legend.
///
/// The legend sits in the frame's top-right corner: one binding per row, and
/// under them a bottom border with the context title centered in it. Every row
/// is painted as it is written in [`legend`](binding::legend), border strokes
/// and all, so the box needs no drawing arithmetic. It paints nothing when the
/// frame cannot hold the legend whole: a legend clipped to fit would show chords
/// without their labels.
#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    /// Paints the legend for `context` into the frame's top-right corner.
    pub fn render(&self, context: Context, frame: &mut tuinix::Frame) {
        let legend = binding::legend_size(context, frame.size());
        if legend != full_size(context) {
            return;
        }

        let style = tuinix::Style::new();
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
