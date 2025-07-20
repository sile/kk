use std::fmt::Write;

use orfail::OrFail;

use crate::{buffer::TextLine, mame::TerminalFrame, state::State};

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
                self.render_line(line, state.viewport.col, frame)?;
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
    ) -> orfail::Result<()> {
        // Skip characters before the viewport's left edge
        for (current_col, ch) in line.char_cols() {
            if current_col >= start_col {
                write!(frame, "{}", ch).or_fail()?;
            }
        }
        Ok(())
    }
}
