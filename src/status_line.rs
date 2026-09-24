//! Paints the status line.

use crate::terminal::put_str;

use crate::state::State;

/// Paints the file path, cursor position, and clipboard summary.
///
/// The line reads `* [PATH:ROW(ROWS):COL(COLS)] CLIPBOARD`, where the leading
/// `*` marks a buffer with unsaved edits and the row, column, and totals are
/// 1-based. The whole row is padded so the reverse-video style reaches the
/// right edge.
#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    /// Paints the status line into `frame`.
    pub fn render(&self, state: &State, frame: &mut tuinix::Frame) {
        let style = tuinix::Style::new().reverse().bold();

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
        put_str(frame, tuinix::Position::ORIGIN, &padded, style);
    }
}
