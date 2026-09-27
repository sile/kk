//! Paints the key-binding legend.
//!
//! See [`render_legend`] for what the box looks like and when it is left off.

use crate::binding::Mode;
use crate::terminal::put_str;

/// Paints the key-binding legend.
///
/// The legend sits in the frame's top-right corner: one binding per row, and
/// under them a bottom border with the mode title centered in it. Every row
/// is painted as it is written in [`legend`](Mode::legend), border strokes
/// and all, so the box needs no drawing arithmetic. It paints nothing when the
/// frame cannot hold the legend whole: a legend clipped to fit would show chords
/// without their labels.
///
/// The legend is drawn into a frame of its own and then pasted in whole, so the
/// cells its rows leave unwritten are painted as blanks. Drawing the rows
/// straight into the frame would leave whatever was underneath showing through
/// the gaps beside them.
pub fn render_legend(mode: Mode, frame: &mut tuinix::Frame) {
    let legend = mode.legend_size(frame.size());
    if legend != full_legend_size(mode) {
        return;
    }

    let mut box_frame = tuinix::Frame::new(legend);
    for (row, text) in mode.legend().iter().enumerate() {
        let at = tuinix::Position { row, col: 0 };
        put_str(&mut box_frame, at, text, tuinix::Style::new());
    }

    let origin = tuinix::Position {
        row: 0,
        col: frame.size().cols - legend.cols,
    };
    frame.put_frame(origin, &box_frame);
}

/// The size the legend of `mode` needs, with room to spare.
pub fn full_legend_size(mode: Mode) -> tuinix::Size {
    mode.legend_size(tuinix::Size {
        rows: usize::MAX,
        cols: usize::MAX,
    })
}
