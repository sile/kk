//! The in-buffer search prompt and its highlight.

use crate::terminal::{char_cols, put_str};

use crate::{
    action::GrepAction,
    buffer::{TextBuffer, TextPosition},
    state::State,
};

/// A search in progress.
///
/// The query is edited as a list of characters with an insertion cursor, so a
/// query can be built up one keypress at a time before it is run.
#[derive(Debug)]
pub struct GrepMode {
    /// The direction a run of the query searches in.
    pub action: GrepAction,

    /// The query as entered so far.
    pub query: Vec<char>,

    /// The index in [`query`](GrepMode::query) where the next character is
    /// inserted.
    pub cursor: usize,
}

impl GrepMode {
    /// Starts an empty query in the direction given by `action`.
    pub fn new(action: GrepAction) -> Self {
        Self {
            action,
            query: Vec::new(),
            cursor: 0,
        }
    }

    /// Returns where the query's insertion cursor belongs inside `region`.
    ///
    /// The position accounts for the prompt's width and for every character
    /// before the cursor.
    pub fn cursor_position(&self, region: tuinix::Region) -> tuinix::Position {
        let mut pos = region.position;
        pos.col += crate::terminal::str_cols(PROMPT);
        for ch in self.query.iter().take(self.cursor) {
            pos.col += char_cols(*ch);
        }
        pos
    }

    /// Inserts `ch` at the query cursor.
    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Runs the query against `buffer` and returns every match.
    ///
    /// An empty query matches nothing.
    pub fn grep(&mut self, buffer: &TextBuffer) -> Highlight {
        if self.query.is_empty() {
            return Highlight::default();
        }

        Highlight::search(buffer, &self.query)
    }
}

/// One matched range in the buffer.
///
/// `start_position` is inclusive and `end_position` is exclusive, so the two
/// compare directly as a half-open range.
#[derive(Debug, Clone, Copy)]
pub struct HighlightItem {
    /// The first position of the match.
    pub start_position: TextPosition,

    /// The position just past the match.
    pub end_position: TextPosition,
}

/// Every match of a query.
#[derive(Debug, Default)]
pub struct Highlight {
    /// The matched ranges, in buffer order.
    pub items: Vec<HighlightItem>,
}

impl Highlight {
    /// Searches `buffer` for `query` (case-insensitive) and returns the
    /// positions of every match.
    fn search(buffer: &TextBuffer, query: &[char]) -> Self {
        let query_lower: Vec<char> = query.iter().flat_map(|c| c.to_lowercase()).collect();
        let query_len = query_lower.len();
        let mut items = Vec::new();

        for (row, line) in buffer.text.iter().enumerate() {
            // (display column, lowercased char) for every character in the line.
            let mut line_end_col = 0;
            let chars: Vec<(usize, char)> = line
                .char_cols()
                .flat_map(|(col, ch)| {
                    line_end_col = col + crate::terminal::char_cols(ch);
                    ch.to_lowercase().map(move |lc| (col, lc))
                })
                .collect();

            if query_len == 0 || chars.len() < query_len {
                continue;
            }

            for start in 0..=(chars.len() - query_len) {
                if chars[start..start + query_len]
                    .iter()
                    .map(|&(_, c)| c)
                    .eq(query_lower.iter().copied())
                {
                    let start_col = chars[start].0;
                    let end_col = chars
                        .get(start + query_len)
                        .map(|&(col, _)| col)
                        .unwrap_or(line_end_col);
                    items.push(HighlightItem {
                        start_position: TextPosition {
                            row,
                            col: start_col,
                        },
                        end_position: TextPosition { row, col: end_col },
                    });
                }
            }
        }

        Self { items }
    }

    /// Returns `true` if `pos` falls inside one of the matches.
    pub fn contains(&self, pos: TextPosition) -> bool {
        self.items
            .iter()
            .any(|item| item.start_position <= pos && pos < item.end_position)
    }
}

/// Prompt shown in the grep/query input line.
const PROMPT: &str = "Search: ";

/// Paints the search prompt and the query typed so far.
#[derive(Debug)]
pub struct GrepQueryRenderer;

impl GrepQueryRenderer {
    /// Paints the prompt into `frame`.
    ///
    /// # Panics
    ///
    /// Panics if [`State::grep_mode`] is `None`, since there is then no query
    /// to paint.
    pub fn render(&self, state: &State, frame: &mut tuinix::Frame) {
        let Some(grep) = &state.grep_mode else {
            unreachable!();
        };

        let query: String = grep.query.iter().collect();
        let text = format!("{PROMPT}{query}");
        put_str(frame, tuinix::Position::ORIGIN, &text, tuinix::Style::new());
    }
}
