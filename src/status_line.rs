use crate::terminal::put_str;
use tuinix::{Frame, Position, Style};

use crate::state::State;

#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    pub fn render(&self, state: &State, frame: &mut Frame) {
        let style = Style::new().reverse().bold();

        let dirty = if state.buffer.dirty { '*' } else { ' ' };
        let path = state.path.display();
        let cursor = state.cursor_position();
        let row = cursor.row + 1; // Convert to 1-based index
        let col = cursor.col + 1; // Convert to 1-based index
        let rows = state.buffer.rows();
        let cols = state.buffer.cols(cursor.row);
        let clipboard = if state.clipboard.summary_line.is_empty() {
            ""
        } else {
            "📋"
        };
        let text = format!(
            " {dirty} [{path}:{row}({rows}):{col}({cols})] {clipboard}{}",
            state.clipboard.summary_line,
        );
        // Pad the whole row so the reverse style covers it.
        let padded = format!("{text:<width$}", width = frame.size().cols);
        put_str(frame, Position::ORIGIN, &padded, style);
    }
}
