use std::fmt::Write;

use crate::terminal::UnicodeTerminalFrame as TerminalFrame;
use crate::error::Result;

use crate::state::State;

#[derive(Debug)]
pub struct MessageLineRenderer;

impl MessageLineRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> Result<()> {
        let Some(message) = &state.message else {
            return Ok(());
        };
        write!(frame, "{message}")?;
        Ok(())
    }
}
