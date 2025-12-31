use std::fmt::Write;

use mame::terminal::UnicodeTerminalFrame as TerminalFrame;
use orfail::OrFail;
use tuinix::TerminalStyle;

use crate::{buffer::TextLine, buffer::TextPosition, state::State};

#[derive(Debug)]
pub struct TextAreaRenderer;

impl TextAreaRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let size = frame.size();
        let available_rows = size.rows;

        // Render visible lines from the buffer starting at viewport position
        let start_row = state.viewport.row;
        let end_row = (start_row + available_rows).min(state.buffer.text.len());

        for (screen_row, buffer_row) in (start_row..end_row).enumerate() {
            if screen_row > 0 {
                writeln!(frame).or_fail()?;
            }
            if let Some(line) = state.buffer.text.get(buffer_row) {
                self.render_line(line, state.viewport.col, frame, state, buffer_row)?;
            }
        }

        // Fill remaining rows with empty lines if needed
        let rendered_rows = end_row.saturating_sub(start_row);
        for _ in rendered_rows..available_rows {
            writeln!(frame).or_fail()?;
        }

        Ok(())
    }

    fn render_line(
        &self,
        line: &TextLine,
        start_col: usize,
        frame: &mut TerminalFrame,
        state: &State,
        line_row: usize,
    ) -> orfail::Result<()> {
        // Calculate marked region for this line if mark is active
        let marked_region = if let Some(mark_pos) = state.mark {
            let cursor_pos = state.cursor_position();
            self.calculate_line_marked_region(mark_pos, cursor_pos, line_row)
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

                let mut style = TerminalStyle::new();
                if is_marked {
                    style = style.reverse();
                }
                if is_highlighted {
                    style = style.bg_color(tuinix::TerminalColor::new(220, 220, 220));
                }
                if pos == state.cursor {
                    if state.grep_mode.is_some() {
                        style = style.underline().bold();
                    } else {
                        style = style.underline();
                    }
                }
                if style != TerminalStyle::RESET {
                    let reset = TerminalStyle::RESET;
                    write!(frame, "{style}{ch}{reset}").or_fail()?;
                } else {
                    write!(frame, "{ch}").or_fail()?;
                }
            }
        }
        Ok(())
    }

    /// Calculate the marked region (start_col, end_col) for a specific line
    fn calculate_line_marked_region(
        &self,
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
}
