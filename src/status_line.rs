//! Paints the status line.

use crate::terminal::put_str;

use crate::state::State;

/// Paints the file path, cursor position, search progress, and clipboard
/// summary.
///
/// The line reads `[PATH:ROW:COL] HITS CLIPBOARD`, where the row and column are
/// 1-based. `HITS` is `n/total` while a search prompt is open: `total` is how
/// many matches the query found and `n` is how many start at or before the
/// cursor, so it names the match the cursor has reached. An empty query counts
/// `0/0`. The whole row is padded so the reverse-video style reaches the right
/// edge.
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
        let hits = if state.search_mode.is_none() {
            String::new()
        } else {
            format!(
                "{}/{}",
                state.highlight.count_up_to(cursor),
                state.highlight.items.len()
            )
        };
        let text = format!(
            " [{path}:{row}:{col}] {hits} {clipboard}{}",
            state.clipboard.summary_line,
        );
        // Pad the whole row so the reverse style covers it.
        let padded = format!("{text:<width$}", width = frame.size().cols);
        put_str(frame, tuinix::Position::ORIGIN, &padded, style);
    }
}
