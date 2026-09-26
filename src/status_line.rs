//! Paints the status line.

use crate::terminal::put_str;

use crate::state::State;

/// Paints the file path, cursor position, search progress, and clipboard
/// summary.
///
/// The line reads `[PATH:ROW:COL] HITS CLIPBOARD`, where the row and column are
/// 1-based. `HITS` is `🔍n/total` while a search prompt is open: `total` is how
/// many matches the query found and `n` is how many start at or before the
/// cursor, so it names the match the cursor has reached. An empty query counts
/// `0/0`. `CLIPBOARD` is a `📋` that is always shown, followed by the summary of
/// whichever clipboard the current context owns: the search prompt's own while
/// a prompt is open, and the buffer's otherwise. The whole row is padded so the
/// reverse-video style reaches the right edge.
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

        // The prompt keeps its own clipboard, so the summary shown is the one
        // belonging to the context that is on screen.
        let summary = if state.search_mode.is_some() {
            &state.search_clipboard.summary_line
        } else {
            &state.clipboard.summary_line
        };

        // The hits are shown only while a prompt is open, and the icon goes with
        // them; the clipboard icon is always there, its summary or not.
        let hits = if state.search_mode.is_none() {
            String::new()
        } else {
            format!(
                "🔍{}/{}",
                state.highlight.count_up_to(cursor),
                state.highlight.items.len()
            )
        };
        let text = format!(" [{path}:{row}:{col}] {hits} 📋{summary}");
        // Pad the whole row so the reverse style covers it. The padding is
        // measured in columns, not `char`s: the icons are wide, so a row padded
        // by character count would stop short of the right edge.
        let width = frame.size().cols;
        let pad = width.saturating_sub(crate::terminal::str_cols(&text));
        let padded = format!("{text}{}", " ".repeat(pad));
        put_str(frame, tuinix::Position::ORIGIN, &padded, style);
    }
}
