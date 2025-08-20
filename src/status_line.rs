use std::fmt::Write;

use mame::UnicodeTerminalFrame as TerminalFrame;
use orfail::OrFail;

use crate::config::Config;
use crate::state::State;

#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    pub fn render(
        &self,
        state: &State,
        config: &Config,
        frame: &mut TerminalFrame,
    ) -> orfail::Result<()> {
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
        let context = config.current_context();
        write!(
            frame,
            "{style} {dirty} [{path}:{row}({rows}):{col}({cols})] {context} | {}{}{reset}",
            state.clipboard.summary_line, filler
        )
        .or_fail()?;

        Ok(())
    }
}
