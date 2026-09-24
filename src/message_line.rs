use crate::error::Result;
use crate::terminal::put_str;
use tuinix::{Frame, Position, Style};

use crate::state::State;

#[derive(Debug)]
pub struct MessageLineRenderer;

impl MessageLineRenderer {
    pub fn render(&self, state: &State, frame: &mut Frame) -> Result<()> {
        let Some(message) = &state.message else {
            return Ok(());
        };
        put_str(frame, Position::ORIGIN, message, Style::new());
        Ok(())
    }
}
