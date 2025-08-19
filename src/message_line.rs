use std::fmt::Write;

use mame::UnicodeTerminalFrame as TerminalFrame;
use orfail::OrFail;

use crate::state::State;

#[derive(Debug)]
pub struct MessageLineRenderer;

impl MessageLineRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let Some(message) = &state.message else {
            return Ok(());
        };
        write!(frame, "{message}").or_fail()?;
        Ok(())
    }
}
