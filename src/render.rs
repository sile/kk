//! Paints the editor's state into a frame.
//!
//! Every renderer takes the state -- plus the few things the core does not
//! hold, like the display path and the mode -- and draws one part of the
//! screen. They are free functions rather than types: none of them carries
//! state between calls, so there is nothing for a struct to hold.

use crate::{
    binding::Mode,
    buffer::{TextLine, TextPosition},
    state::State,
    terminal::put_str,
};

/// Paints the visible slice of the buffer, with the mark and search highlight.
///
/// A marked range and the character the cursor sits on while a search prompt is
/// open are both reversed, so the cursor stands out as plainly as a mark. A
/// matched range is bold and underlined instead, so a hit and a mark cannot be
/// taken for one another.
pub fn render_text_area(state: &State, frame: &mut tuinix::Frame) {
    let available_rows = frame.size().rows;

    // Render visible lines from the buffer starting at viewport position
    let start_row = state.viewport.row;
    let end_row = (start_row + available_rows).min(state.buffer.rows());

    for (screen_row, buffer_row) in (start_row..end_row).enumerate() {
        if let Some(line) = state.buffer.line(buffer_row) {
            render_line(
                line,
                state.viewport.col,
                frame,
                state,
                buffer_row,
                screen_row,
            );
        }
    }
}

fn render_line(
    line: &TextLine,
    start_col: usize,
    frame: &mut tuinix::Frame,
    state: &State,
    line_row: usize,
    screen_row: usize,
) {
    // Calculate marked region for this line if mark is active
    let marked_region = if let Some(mark_pos) = state.mark {
        let cursor_pos = state.cursor_position();
        calculate_line_marked_region(mark_pos, cursor_pos, line_row)
    } else {
        None
    };

    // Skip characters before the viewport's left edge and render with marking
    for (current_col, ch) in line.char_cols() {
        if current_col >= start_col {
            let pos = TextPosition {
                row: line_row,
                col: current_col,
            };

            let is_marked = marked_region
                .as_ref()
                .is_some_and(|(start, end)| current_col >= *start && current_col < *end);
            let is_highlighted = state.highlight.contains(pos);
            let is_cursor = state.search_prompt.is_some() && pos == state.cursor;

            let style = if is_cursor || is_marked {
                tuinix::Style::new().reverse()
            } else if is_highlighted {
                tuinix::Style::new().bold().underline()
            } else {
                tuinix::Style::new()
            };
            let at = tuinix::Position {
                row: screen_row,
                col: current_col - start_col,
            };
            put_str(frame, at, &ch.to_string(), style);
        }
    }
}

/// Calculate the marked region (start_col, end_col) for a specific line
fn calculate_line_marked_region(
    mark_pos: TextPosition,
    cursor_pos: TextPosition,
    line_row: usize,
) -> Option<(usize, usize)> {
    // Determine selection bounds (mark and cursor can be in any order)
    let (start_pos, end_pos) = if mark_pos <= cursor_pos {
        (mark_pos, cursor_pos)
    } else {
        (cursor_pos, mark_pos)
    };

    // Check if this line is within the marked region
    if line_row < start_pos.row || line_row > end_pos.row {
        return None;
    }

    let start_col = if line_row == start_pos.row {
        start_pos.col
    } else {
        0
    };

    let end_col = if line_row == end_pos.row {
        end_pos.col
    } else {
        // Mark to end of line - use a large number or get actual line length
        usize::MAX
    };

    // Only return a region if there's actually something to mark
    if start_col < end_col {
        Some((start_col, end_col))
    } else {
        None
    }
}

/// Paints the key-binding legend.
///
/// The legend sits in the frame's top-right corner: one binding per row, and
/// under them a bottom border with the mode title centered in it. Every row
/// is painted as it is written in [`legend()`](Mode::legend), border strokes
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

/// Paints the file path, cursor position, search progress, and clipboard
/// summary.
///
/// The line reads `[PATH:ROW:COL] HITS CLIPBOARD`, where the row and column are
/// 1-based. `HITS` is `🔍n/total` while a search prompt is open: `total` is how
/// many matches the query found and `n` is how many start at or before the
/// cursor, so it names the match the cursor has reached. An empty query counts
/// `0/0`. `CLIPBOARD` is a `📋` that is always shown, followed by the summary of
/// whichever clipboard the current mode owns: the search prompt's own while
/// a prompt is open, and the buffer's otherwise. The whole row is padded so the
/// reverse-video style reaches the right edge.
///
/// `path` is passed in rather than read off [`State`]: the core holds no file
/// path, so the edge that owns one hands its display form over for painting.
pub fn render_status_line(state: &State, path: &str, frame: &mut tuinix::Frame) {
    let style = tuinix::Style::new().reverse().bold();

    let cursor = state.cursor_position();
    let row = cursor.row + 1; // Convert to 1-based index
    let col = cursor.col + 1; // Convert to 1-based index

    // The prompt keeps its own clipboard, so the summary shown is the one
    // belonging to the mode that is on screen.
    let summary = if state.search_prompt.is_some() {
        state.search_clipboard.summary_line()
    } else {
        state.clipboard.summary_line()
    };

    // The hits are shown only while a prompt is open, and the icon goes with
    // them; the clipboard icon is always there, its summary or not. The
    // separator goes in with the hits so an empty `hits` leaves one space
    // rather than two.
    let hits = if state.search_prompt.is_none() {
        String::new()
    } else {
        format!(
            " 🔍{}/{}",
            state.highlight.count_up_to(cursor),
            state.highlight.items.len()
        )
    };
    let text = format!(" [{path}:{row}:{col}]{hits} 📋{summary}");
    // Pad the whole row so the reverse style covers it. The padding is
    // measured in columns, not `char`s: the icons are wide, so a row padded
    // by character count would stop short of the right edge.
    let width = frame.size().cols;
    let pad = width.saturating_sub(crate::terminal::str_cols(&text));
    let padded = format!("{text}{}", " ".repeat(pad));
    put_str(frame, tuinix::Position::ORIGIN, &padded, style);
}

/// Paints the bottom row of the editor.
///
/// While a search prompt is open this row holds the prompt and the query typed
/// so far, and any pending [`State::message`] is left unpainted: the cursor
/// sits in the query, so the prompt has to stay visible. Otherwise the row
/// holds the message, and a frame with neither is left untouched.
pub fn render_message_line(state: &State, frame: &mut tuinix::Frame) {
    // The prompt and the query are not one string on `State`, so the line
    // they make is built here.
    let search_line = state.search_prompt.as_ref().map(|search| search.line());
    let text = match &search_line {
        Some(line) => line.as_str(),
        None => match &state.message {
            Some(message) => message.as_str(),
            None => return,
        },
    };
    put_str(frame, tuinix::Position::ORIGIN, text, tuinix::Style::new());
}
