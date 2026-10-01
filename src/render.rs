//! Paints the editor's state into a frame.
//!
//! Every renderer takes the state -- plus the few things the core does not
//! hold, like the display path and the mode -- and draws one part of the
//! screen. They are free functions rather than types: none of them carries
//! state between calls, so there is nothing for a struct to hold.

use crate::{
    binding::Mode,
    buffer::{TextLine, TextPosition},
    state::{HIT_GUTTER_COLS, State},
    terminal::put_str,
};

/// Paints the visible slice of the buffer, with the mark and search highlight.
///
/// A marked range and the character the cursor sits on while a search prompt is
/// open are both reversed, so the cursor stands out as plainly as a mark. A
/// matched range is bold and underlined instead, so a hit and a mark cannot be
/// taken for one another.
///
/// The reverse is painted wherever the cursor sits, including one column past
/// the last character: there is no character to paint there, so a reversed blank
/// marks the cell instead. A cursor at the end of a line is a position the
/// buffer really reaches -- `End`, `C-e`, and a hit or a click at the line's end
/// all arrive there -- so it must be as visible as one mid-line.
///
/// While a search prompt is open a gutter is reserved on the left, showing how
/// many hits each visible line holds and, above and below, how many sit outside
/// the visible slice; the text shifts right by the gutter's width to make room.
/// The gutter is drawn from the same walk as the text, so the two cannot
/// disagree about which rows are visible. When the prompt is closed the gutter
/// is gone and the text starts at the frame's left edge as before.
///
/// A summary row takes a row of its own, above the first visible line and below
/// the last, so the text loses that row's height while a summary is shown. The
/// totals are measured over the text area's full height, before any summary is
/// drawn, so a summary's presence never changes the totals. The viewport, though,
/// is scrolled against the rows the text is *drawn* in -- the full height less
/// one row per summary -- so the cursor lands in a row a summary cannot cover;
/// [`State::text_rows()`] gives both this renderer and
/// [`adjust_viewport()`](State::adjust_viewport) that height, so the two cannot
/// disagree about it.
pub fn render_text_area(state: &State, frame: &mut tuinix::Frame) {
    let available_rows = frame.size().rows;

    let start_row = state.viewport.row;

    // A summary row costs a row of the text's height, so the text is clipped by
    // as many rows as are shown above and below it. `State::text_rows()` settles
    // that height against the totals, because each decides the other: the totals
    // are measured over the drawn slice, and a summary drawn costs the text a
    // row.
    let gutter_shown = state.search_prompt.is_some();
    let text_height = state.text_rows(available_rows);
    let end_row = (start_row + text_height).min(state.buffer.rows());

    // The totals are measured over that same drawn slice, so every row the loop
    // below walks is either drawn beside its own count or is on the far side of
    // one of the two totals. A row between the last drawn line and the bottom
    // total would be counted nowhere.
    let (above, below) = if gutter_shown {
        state.hits_outside(start_row, end_row)
    } else {
        (0, 0)
    };

    // The top summary claims row 0 and the bottom the last row. On a frame of a
    // single row they would collide, so the top wins: a one-row frame has no
    // room for two summaries, and the nearer edge is the more useful one.
    let top_summary = above > 0 && start_row > 0;
    if top_summary {
        render_gutter_row(frame, 0, above, GUTTER_SEPARATOR_TOTAL, false);
    }
    if below > 0 && available_rows.saturating_sub(1) > 0 {
        let last = available_rows - 1;
        render_gutter_row(frame, last, below, GUTTER_SEPARATOR_TOTAL, false);
    }

    for (screen_row, buffer_row) in (start_row..end_row).enumerate() {
        if let Some(line) = state.buffer.line(buffer_row) {
            let top = usize::from(top_summary);
            let gutter = gutter_shown.then(|| GutterCell {
                count: count_hits_on_row(state, buffer_row),
                // The cursor's row is reversed, so the gutter marks the cursor
                // as plainly as the reversed line text beside it does.
                reversed: state.cursor.row == buffer_row,
            });
            render_line(line, frame, state, buffer_row, top + screen_row, gutter);
        }
    }
}

/// The gutter cell of one buffer line.
struct GutterCell {
    /// How many hits start on the line.
    count: usize,

    /// Whether to reverse the cell because the cursor is on the line.
    reversed: bool,
}

/// Returns how many hits start on `row`.
///
/// A hit is counted on the row it starts on, matching how the text is
/// highlighted, so a hit that wraps is counted once.
fn count_hits_on_row(state: &State, row: usize) -> usize {
    state.highlight.count_on_row(row)
}

/// The separator drawn on a buffer line's gutter row.
const GUTTER_SEPARATOR_LINE: char = '|';

/// The separator drawn on a summary row, which a reader can tell from a buffer
/// line by its shape alone: the two kinds of row are otherwise the same width.
const GUTTER_SEPARATOR_TOTAL: char = ':';

/// Writes a gutter row -- the count cell and the separator -- at `screen_row`,
/// columns `0..HIT_GUTTER_COLS`, reversing the count cell when `reverse` is set.
///
/// The count cell is three columns wide: two digits and the `+` that fills the
/// third only when they overflow, so a count past `99` reads `99+` and the cell
/// never grows. A zero count leaves the cell blank, the way a line-number gutter
/// says zero with no digits at all. A space follows the separator; it is part of
/// the gutter's columns but nothing has to be written to it.
fn render_gutter_row(
    frame: &mut tuinix::Frame,
    screen_row: usize,
    count: usize,
    separator: char,
    reverse: bool,
) {
    let cell = if count == 0 {
        "   ".to_string()
    } else if count > 99 {
        "99+".to_string()
    } else {
        format!("{count:>2} ")
    };
    let cell_style = if reverse {
        tuinix::Style::new().reverse()
    } else {
        tuinix::Style::new()
    };
    let at = tuinix::Position {
        row: screen_row,
        col: 0,
    };
    put_str(frame, at, &cell, cell_style);

    let sep_at = tuinix::Position {
        row: screen_row,
        col: 3,
    };
    put_str(frame, sep_at, &separator.to_string(), tuinix::Style::new());
}

fn render_line(
    line: &TextLine,
    frame: &mut tuinix::Frame,
    state: &State,
    line_row: usize,
    screen_row: usize,
    gutter: Option<GutterCell>,
) {
    // The gutter, when drawn, takes the left of the row, and the text starts
    // just past it: the text's left edge is the gutter's width or nothing.
    let text_offset = if let Some(cell) = gutter {
        render_gutter_row(
            frame,
            screen_row,
            cell.count,
            GUTTER_SEPARATOR_LINE,
            cell.reversed,
        );
        HIT_GUTTER_COLS
    } else {
        0
    };
    let start_col = state.viewport.col;

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
                col: text_offset + (current_col - start_col),
            };
            put_str(frame, at, &ch.to_string(), style);
        }
    }

    // The character loop only runs for columns that hold a character, so the
    // end-of-line position -- one column past the last character, where there
    // is nothing to reverse -- would leave the cursor invisible. Paint a
    // reversed blank there instead, the way an editor shows a block cursor
    // past the last character. A mark already covers this column: its end is
    // `usize::MAX` for a range that runs to the line's end.
    //
    // No left-edge test is needed: the viewport's column never scrolls past a
    // line's width, so a column at or past the width is at or past the edge
    // too.
    if state.search_prompt.is_some() && state.cursor.row == line_row {
        let cursor_col = state.cursor.col;
        let line_width = line.width();
        if cursor_col >= line_width {
            let at = tuinix::Position {
                row: screen_row,
                col: text_offset + (cursor_col - start_col),
            };
            put_str(frame, at, " ", tuinix::Style::new().reverse());
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
            state.highlight.len()
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
