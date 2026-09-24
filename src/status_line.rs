use std::fmt::Write;

use crate::terminal::UnicodeTerminalFrame as TerminalFrame;
use crate::error::Result;

use crate::state::State;

#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> Result<()> {
        let style = tuinix::TerminalStyle::new().reverse().bold();
        let reset = tuinix::TerminalStyle::RESET;
        let filler = " ".repeat(frame.size().cols);

        let dirty = if state.buffer.dirty { '*' } else { ' ' };
        let path = state.path.display();
        let cursor = state.cursor_position();
        let row = cursor.row + 1; // Convert to 1-based index
        let col = cursor.col + 1; // Convert to 1-based index
        let rows = state.buffer.rows();
        let cols = state.buffer.cols(cursor.row);
        write!(
            frame,
            "{style} {dirty} [{path}:{row}({rows}):{col}({cols})] {}{}{filler}{reset}",
            if state.clipboard.summary_line.is_empty() {
                ""
            } else {
                "📋"
            },
            state.clipboard.summary_line,
        )
        ?;

        Ok(())
    }
}
