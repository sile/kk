use std::fmt::Write;

use orfail::OrFail;

use crate::{mame::TerminalFrame, state::State};

#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let style = tuinix::TerminalStyle::new().reverse().bold();
        let reset = tuinix::TerminalStyle::RESET;
        let filler = " ".repeat(frame.size().cols);

        let dirty = if state.buffer.dirty { '*' } else { ' ' };
        let path = state.path.display();
        let cursor = state.terminal_cursor_position();
        let row = cursor.row + 1; // Convert to 1-based index
        let col = cursor.col + 1; // Convert to 1-based index
        let context = state.context.group_path();
        write!(
            frame,
            "{style} {dirty} [{path}:{row}:{col}] {context}{}{reset}",
            filler
        )
        .or_fail()?;

        Ok(())
    }
}
