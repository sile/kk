//! Paints the buffer text.

use crate::terminal::put_str;

use crate::{buffer::TextLine, buffer::TextPosition, state::State};

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
