use crate::terminal::{char_cols, put_str};
use tuinix::{Frame, Position, Region, Style};

use crate::{
    action::GrepAction,
    buffer::{TextBuffer, TextPosition},
    state::State,
};

#[derive(Debug)]
pub struct GrepMode {
    pub action: GrepAction,
    pub query: Vec<char>,
    pub cursor: usize,
}

impl GrepMode {
    pub fn new(action: GrepAction) -> Self {
        Self {
            action,
            query: Vec::new(),
            cursor: 0,
        }
    }

    pub fn cursor_position(&self, region: Region) -> Position {
        let mut pos = region.position;
        pos.col += crate::terminal::str_cols(PROMPT);
        for ch in self.query.iter().take(self.cursor) {
            pos.col += char_cols(*ch);
        }
        pos
    }

    pub fn handle_char_insert(&mut self, key: tuinix::KeyInput) {
        let tuinix::KeyCode::Char(ch) = key.code else {
            return;
        };
        self.query.insert(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn grep(&mut self, buffer: &TextBuffer) -> Highlight {
        if self.query.is_empty() {
            return Highlight::default();
        }

        Highlight::search(buffer, &self.query)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HighlightItem {
    pub start_position: TextPosition,
    pub end_position: TextPosition,
}

#[derive(Debug, Default)]
pub struct Highlight {
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

    pub fn contains(&self, pos: TextPosition) -> bool {
        self.items
            .iter()
            .any(|item| item.start_position <= pos && pos < item.end_position)
    }
}

/// Prompt shown in the grep/query input line.
const PROMPT: &str = "Search: ";

#[derive(Debug)]
pub struct GrepQueryRenderer;

impl GrepQueryRenderer {
    pub fn render(&self, state: &State, frame: &mut Frame) {
        let Some(grep) = &state.grep_mode else {
            unreachable!();
        };

        let query: String = grep.query.iter().collect();
        let text = format!("{PROMPT}{query}");
        put_str(frame, Position::ORIGIN, &text, Style::new());
    }
}
