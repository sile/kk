//! The in-buffer search prompt and its highlight.

use crate::{
    buffer::{TextBuffer, TextPosition},
    terminal::char_cols,
};

/// An open search prompt and the query typed into it.
///
/// The query is edited as a list of characters with an insertion cursor, so a
/// query can be built up one keypress at a time before it is run. The matches
/// are collected by [`search`](SearchPrompt::search) in buffer order; which one
/// the cursor visits is the hit commands' business, not the prompt's, so the
/// prompt holds no direction.
#[derive(Debug, Default)]
pub struct SearchPrompt {
    /// The query as entered so far.
    query: Vec<char>,

    /// The index in `query` where the next character is inserted.
    cursor: usize,
}

impl SearchPrompt {
    /// Starts an empty query.
    pub fn new() -> Self {
        Self::default()
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

    /// Returns the line to show while the query is being typed: the prompt
    /// followed by the query so far.
    pub fn line(&self) -> String {
        format!("{PROMPT}{}", self.query())
    }

    /// Returns the query as a string.
    pub fn query(&self) -> String {
        self.query.iter().collect()
    }

    /// Returns the index in the query where the next character is inserted.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Inserts `ch` at the query cursor.
    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Deletes the character before the query cursor.
    ///
    /// Returns `true` if a character was deleted, or `false` when the cursor
    /// is at the start of the query.
    pub fn delete_char_backward(&mut self) -> bool {
        if self.cursor > 0 {
            self.query.remove(self.cursor - 1);
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    /// Deletes the character under the query cursor.
    ///
    /// Returns `true` if a character was deleted, or `false` when the cursor
    /// is at the end of the query.
    pub fn delete_char_forward(&mut self) -> bool {
        if self.cursor < self.query.len() {
            self.query.remove(self.cursor);
            true
        } else {
            false
        }
    }

    /// Moves the query cursor one character left.
    pub fn move_cursor_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the query cursor one character right.
    pub fn move_cursor_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.query.len());
    }

    /// Moves the query cursor to the start of the query.
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Moves the query cursor to the end of the query.
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.query.len();
    }

    /// Removes and returns the query from the cursor to its end.
    ///
    /// An empty cut returns an empty string.
    pub fn cut_to_end(&mut self) -> String {
        self.query.drain(self.cursor..).collect()
    }

    /// Runs the query against `buffer` and returns every match.
    ///
    /// An empty query matches nothing.
    pub fn search(&self, buffer: &TextBuffer) -> Highlight {
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

/// One buffer row that holds hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HitRun {
    /// The row the hits start on.
    row: usize,

    /// How many hits start on that row.
    count: usize,
}

/// Every match of a query.
#[derive(Debug, Default)]
pub struct Highlight {
    /// The matched ranges, in buffer order.
    items: Vec<HighlightItem>,

    /// The same matches collapsed to one entry per row, in row order, plus the
    /// running total of every count before it.
    ///
    /// The gutter asks for a row's count and for the totals on either side of
    /// the visible slice once per render, and the viewport is scrolled by
    /// repeating that ask until it settles. Walking `items` each time would make
    /// every one of those asks linear in the number of hits, so the runs are
    /// built once here instead and read by binary search: a row's count is the
    /// one run that names it, and a total is a difference of prefix sums.
    runs: Vec<HitRun>,

    /// `prefix[i]` is the number of hits in `runs[..i]`, so `prefix.len()` is
    /// `runs.len() + 1` and `prefix.last()` is the total.
    prefix: Vec<usize>,
}

impl Highlight {
    /// Searches `buffer` for `query` (case-insensitive) and returns the
    /// positions of every match.
    fn search(buffer: &TextBuffer, query: &[char]) -> Self {
        let query_lower: Vec<char> = query.iter().flat_map(|c| c.to_lowercase()).collect();
        let query_len = query_lower.len();
        let mut items = Vec::new();

        for (row, line) in buffer.lines().enumerate() {
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

        Self::new(items)
    }

    /// Builds a highlight from matches in buffer order.
    ///
    /// The matches are in row order -- `search` walks the buffer's lines in
    /// order -- which is what lets [`count_on_row()`](Highlight::count_on_row)
    /// and [`count_outside()`](Highlight::count_outside) binary-search the runs.
    fn new(items: Vec<HighlightItem>) -> Self {
        let mut runs: Vec<HitRun> = Vec::new();
        for item in &items {
            let row = item.start_position.row;
            match runs.last_mut() {
                Some(run) if run.row == row => run.count += 1,
                _ => runs.push(HitRun { row, count: 1 }),
            }
        }

        let mut prefix = Vec::with_capacity(runs.len() + 1);
        prefix.push(0);
        for run in &runs {
            prefix.push(prefix.last().copied().unwrap_or(0) + run.count);
        }

        Self {
            items,
            runs,
            prefix,
        }
    }

    /// The matches, in buffer order.
    pub fn items(&self) -> &[HighlightItem] {
        &self.items
    }

    /// Returns how many matches there are.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the query matched nothing.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the first match, if any.
    pub fn first(&self) -> Option<&HighlightItem> {
        self.items.first()
    }

    /// Returns the last match, if any.
    pub fn last(&self) -> Option<&HighlightItem> {
        self.items.last()
    }

    /// Returns how many hits start on `row`.
    ///
    /// The runs are in row order, so the row's run -- and no other -- is found
    /// by one binary search.
    pub fn count_on_row(&self, row: usize) -> usize {
        match self.runs.binary_search_by_key(&row, |run| run.row) {
            Ok(i) => self.runs[i].count,
            Err(_) => 0,
        }
    }

    /// Returns `true` if `pos` falls inside one of the matches.
    pub fn contains(&self, pos: TextPosition) -> bool {
        self.items
            .iter()
            .any(|item| item.start_position <= pos && pos < item.end_position)
    }

    /// Returns how many matches begin at or before `pos`.
    ///
    /// It counts the match `pos` falls inside as reached, so it is the number
    /// of the match the cursor is on once the cursor is on one, and `0` while
    /// the cursor is still before the first match.
    pub fn count_up_to(&self, pos: TextPosition) -> usize {
        self.items
            .iter()
            .take_while(|item| item.start_position <= pos)
            .count()
    }

    /// Returns how many hits start before `start_row` and how many start at or
    /// after `end_row`.
    ///
    /// The two boundaries are found by binary search and the counts between
    /// them read off the prefix sums, so this costs a logarithm, not a walk.
    pub fn count_outside(&self, start_row: usize, end_row: usize) -> (usize, usize) {
        let before = self.runs.partition_point(|run| run.row < start_row);
        let at_or_after = self.runs.partition_point(|run| run.row < end_row);
        let above = self.prefix.get(before).copied().unwrap_or(0);
        let total = self.prefix.last().copied().unwrap_or(0);
        let below = total - self.prefix.get(at_or_after).copied().unwrap_or(total);
        (above, below)
    }
}

/// Prompt shown in the search/query input line.
const PROMPT: &str = "Search: ";
