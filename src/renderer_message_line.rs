use std::fmt::Write;

use orfail::OrFail;

use crate::{mame::TerminalFrame, state::State};

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
