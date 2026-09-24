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
///
/// The legend is drawn into a frame of its own and then pasted in whole, so the
/// cells its rows leave unwritten are painted as blanks. Drawing the rows
/// straight into the frame would leave whatever was underneath showing through
/// the gaps beside them.
#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    /// Paints the legend for `context` into the frame's top-right corner.
    pub fn render(&self, context: Context, frame: &mut tuinix::Frame) {
        let legend = binding::legend_size(context, frame.size());
        if legend != full_size(context) {
            return;
        }

        let mut box_frame = tuinix::Frame::new(tuinix::Size {
            rows: legend.rows,
            cols: legend.cols,
        });
        for (row, text) in binding::legend(context).iter().enumerate() {
            let at = tuinix::Position { row, col: 0 };
            put_str(&mut box_frame, at, text, tuinix::Style::new());
        }

        let origin = tuinix::Position {
            row: 0,
            col: frame.size().cols - legend.cols,
        };
        frame.put_frame(origin, &box_frame);
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
