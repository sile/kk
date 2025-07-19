use std::fmt::Write;

use orfail::OrFail;

use crate::{buffer::TextLine, mame::TerminalFrame, state::State};

#[derive(Debug)]
pub struct TextAreaRenderer;

impl TextAreaRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let size = frame.size();
        let available_rows = size.rows.saturating_sub(2); // Reserve space for status and message lines

        // Render visible lines from the buffer
        for (row, line) in state.buffer.text.iter().take(available_rows).enumerate() {
            if row > 0 {
                writeln!(frame).or_fail()?;
            }
            self.render_line(line, frame)?;
        }

        // Fill remaining rows with empty lines if needed
        let rendered_rows = state.buffer.text.len().min(available_rows);
        for _ in rendered_rows..available_rows {
            writeln!(frame).or_fail()?;
        }

        Ok(())
    }

    fn render_line(&self, line: &TextLine, frame: &mut TerminalFrame) -> orfail::Result<()> {
        for &ch in &line.0 {
            write!(frame, "{}", ch).or_fail()?;
        }
        Ok(())
    }
}
