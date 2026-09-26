//! Paints the status line.

use crate::terminal::put_str;

use crate::state::State;

/// Paints the file path, cursor position, and clipboard summary.
///
/// The line reads `[PATH:ROW:COL] CLIPBOARD`, where the row and column are
/// 1-based. The whole row is padded so the reverse-video style reaches the
/// right edge.
///
/// `path` is passed in rather than read off [`State`]: the core holds no file
/// path, so the edge that owns one hands its display form over for painting.
#[derive(Debug)]
pub struct StatusLineRenderer;

impl StatusLineRenderer {
    /// Paints the status line into `frame`, labelling the buffer `path`.
    pub fn render(&self, state: &State, path: &str, frame: &mut tuinix::Frame) {
        let style = tuinix::Style::new().reverse().bold();

        let cursor = state.cursor_position();
        let row = cursor.row + 1; // Convert to 1-based index
        let col = cursor.col + 1; // Convert to 1-based index
        let clipboard = if state.clipboard.summary_line.is_empty() {
            ""
        } else {
            "📋"
        };
        let text = format!(
            " [{path}:{row}:{col}] {clipboard}{}",
            state.clipboard.summary_line,
        );
        // Pad the whole row so the reverse style covers it.
        let padded = format!("{text:<width$}", width = frame.size().cols);
        put_str(frame, tuinix::Position::ORIGIN, &padded, style);
    }
}
