//! Paints the message line.

use crate::terminal::put_str;
use tuinix::{Frame, Position, Style};

use crate::state::State;

/// Paints [`State::message`] as the bottom row of the editor.
///
/// A frame with no pending message is left untouched.
#[derive(Debug)]
pub struct MessageLineRenderer;

impl MessageLineRenderer {
    /// Paints the pending message, if any, into `frame`.
    pub fn render(&self, state: &State, frame: &mut Frame) {
        let Some(message) = &state.message else {
            return;
        };
        put_str(frame, Position::ORIGIN, message, Style::new());
    }
}
