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
        let context = state.context.group_path();
        write!(frame, "{style} {dirty} [{path}] {context}{}{reset}", filler).or_fail()?;

        Ok(())
    }
}
