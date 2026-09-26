//! Paints the message line.

use crate::terminal::put_str;

use crate::state::State;

/// Paints the bottom row of the editor.
///
/// While a search prompt is open this row holds the prompt and the query typed
/// so far, and any pending [`State::message`] is left unpainted: the cursor
/// sits in the query, so the prompt has to stay visible. Otherwise the row
/// holds the message, and a frame with neither is left untouched.
#[derive(Debug)]
pub struct MessageLineRenderer;

impl MessageLineRenderer {
    /// Paints the search prompt or the pending message, whichever applies, into
    /// `frame`.
    pub fn render(&self, state: &State, frame: &mut tuinix::Frame) {
        // The prompt and the query are not one string on `State`, so the line
        // they make is built here.
        let search_line = state.search_mode.as_ref().map(|search| search.line());
        let text = match &search_line {
            Some(line) => line.as_str(),
            None => match &state.message {
                Some(message) => message.as_str(),
                None => return,
            },
        };
        put_str(frame, tuinix::Position::ORIGIN, text, tuinix::Style::new());
    }
}
