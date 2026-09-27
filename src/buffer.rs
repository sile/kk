//! The buffer model: lines of characters, with column-aware edits.

use std::sync::Arc;

/// The text being edited.
///
/// A buffer is a list of [`TextLine`]s held in memory. The edits rewrite the
/// lines in place; whether the result has been written out is the edge's
/// business, not the buffer's.
///
/// Rows and columns here are 0-based. A column counts display cells, not
/// characters, so a column must be adjusted to a character boundary before it
/// can address one of the line's characters.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// The lines, in order.
    text: Vec<TextLine>,
}

impl TextBuffer {
    /// Builds a buffer from `text`, splitting it into lines.
    ///
    /// A trailing newline does not produce a final empty line.
    pub fn new(text: &str) -> Self {
        Self {
            text: text
                .lines()
                .map(|l| TextLine(Arc::new(l.chars().collect())))
                .collect(),
        }
    }

    /// Renders the whole buffer, newline-terminated, for writing to a file.
    pub fn to_text(&self) -> String {
        let mut content = self
            .text
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        content.push('\n');
        content
    }

    /// Returns the number of lines.
    pub fn rows(&self) -> usize {
        self.text.len()
    }

    /// Returns line `row`, or `None` if there is no such line.
    pub fn line(&self, row: usize) -> Option<&TextLine> {
        self.text.get(row)
    }

    /// Removes and returns line `row`, or `None` if there is no such line.
    pub fn remove_line(&mut self, row: usize) -> Option<TextLine> {
        if row < self.text.len() {
            Some(self.text.remove(row))
        } else {
            None
        }
    }

    /// Returns the lines, in order.
    pub fn lines(&self) -> std::slice::Iter<'_, TextLine> {
        self.text.iter()
    }

    /// Returns the text between `start` and `end`, or `None` when the range is
    /// empty or has no lines.
    ///
    /// A single-line range is that line's characters from `start.col` up to but
    /// not including `end.col`. A multi-line range is the first line's tail, a
    /// newline, every whole line between, and the last line's head, with a
    /// newline after each line but the last.
    pub fn text_in_range(&self, start: TextPosition, end: TextPosition) -> Option<String> {
        if start == end {
            return None;
        }

        let mut result = String::new();

        if start.row == end.row {
            if let Some(line) = self.line(start.row) {
                for (col, ch) in line.char_cols() {
                    if col >= start.col && col < end.col {
                        result.push(ch);
                    }
                }
            }
        } else {
            for row in start.row..=end.row {
                let Some(line) = self.line(row) else {
                    continue;
                };

                if row == start.row {
                    for (col, ch) in line.char_cols() {
                        if col >= start.col {
                            result.push(ch);
                        }
                    }
                    result.push('\n');
                } else if row == end.row {
                    for (col, ch) in line.char_cols() {
                        if col < end.col {
                            result.push(ch);
                        }
                    }
                } else {
                    result.push_str(&line.to_string());
                    result.push('\n');
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Removes the text between `start` and `end`.
    ///
    /// The lines the range covers are joined where it crosses a line break, so
    /// a multi-line range leaves one line holding the text before `start.col`
    /// followed by the text from `end.col` on. Does nothing when the range is
    /// empty.
    ///
    /// The whole deletion is one unit to the buffer, but not to the undo
    /// history: a caller that wants it undone in one step opens the edit run
    /// around this call.
    pub fn delete_range(&mut self, start: TextPosition, end: TextPosition) {
        if start == end {
            return;
        }

        if start.row == end.row {
            self.remove_cols(start.row, start.col, end.col);
            return;
        }

        // Drop the whole lines between the first and the last, then join the
        // last onto the first: what the first keeps is its head, and what the
        // last keeps is its own tail.
        for _ in start.row + 1..end.row {
            self.remove_line(start.row + 1);
        }

        self.truncate_line(start.row, start.col);

        if let Some(end_line) = self.line(start.row + 1) {
            let chars_to_keep: Vec<char> = end_line
                .char_cols()
                .filter(|(col, _)| *col >= end.col)
                .map(|(_, ch)| ch)
                .collect();

            self.extend_line_from_chars(start.row, chars_to_keep);
        }

        self.remove_line(start.row + 1);
    }

    /// Keeps only the characters of line `row` before column `col`.
    ///
    /// Does nothing if there is no such line.
    pub fn truncate_line(&mut self, row: usize, col: usize) {
        if let Some(line) = self.text.get_mut(row) {
            line.truncate_to_col(col);
        }
    }

    /// Appends `chars` to the end of line `row`.
    ///
    /// Does nothing if there is no such line.
    pub fn extend_line_from_chars(&mut self, row: usize, chars: Vec<char>) {
        if let Some(line) = self.text.get_mut(row) {
            line.extend_from_chars(chars);
        }
    }

    /// Removes the characters of line `row` in the column range
    /// `[start_col, end_col)`.
    ///
    /// Does nothing if there is no such line.
    pub fn remove_cols(&mut self, row: usize, start_col: usize, end_col: usize) {
        if let Some(line) = self.text.get_mut(row) {
            line.remove_cols(start_col, end_col);
        }
    }

    /// Removes and returns the characters of line `row` at or past column
    /// `col`.
    ///
    /// Returns the removed text, or `None` if there is no such line. An empty
    /// cut returns `Some(String::new())`.
    pub fn cut_line_tail(&mut self, row: usize, col: usize) -> Option<String> {
        let line = self.text.get_mut(row)?;
        let tail = line.split_off_at_col(col);
        Some(tail.into_iter().collect())
    }

    /// Joins line `row + 1` onto the end of line `row`, removing the former.
    ///
    /// Does nothing if either line is missing.
    pub fn join_next_line(&mut self, row: usize) {
        if row + 1 < self.text.len() {
            let next_line = self.text.remove(row + 1);
            if let Some(line) = self.text.get_mut(row) {
                line.extend_from_line(next_line);
            }
        }
    }

    /// Returns the display width of line `row`, or 0 if there is no such line.
    pub fn cols(&self, row: usize) -> usize {
        self.text.get(row).map(|l| l.cols()).unwrap_or_default()
    }

    /// Moves `pos`'s column onto a character boundary.
    ///
    /// A column can fall inside a wide character (or between the parts of a
    /// grapheme). With `floor`, the column snaps back to the start of the
    /// character it landed in; otherwise it snaps forward past it. A row that
    /// does not exist yields column 0.
    pub fn adjust_to_char_boundary(&self, mut pos: TextPosition, floor: bool) -> TextPosition {
        if let Some(line) = self.text.get(pos.row) {
            pos.col = line.adjust_to_char_boundary(pos.col, floor);
        } else {
            pos.col = 0;
        }
        pos
    }

    /// Deletes the character at `pos`.
    ///
    /// At the end of a line, the next line is joined onto this one. Returns
    /// `true` if anything was deleted.
    pub fn delete_char_at(&mut self, pos: TextPosition) -> bool {
        // Store the character for undo before deleting
        if let Some(line) = self.text.get(pos.row)
            && let Some(_ch) = line.char_at_col(pos.col)
        {
            self.delete_char_at_internal(pos);
            return true;
        }

        // Handle forward delete at line end (merge with next line)
        if pos.col >= self.cols(pos.row) && pos.row < self.text.len().saturating_sub(1) {
            let deleted_line = self.text.get(pos.row + 1).cloned();
            if let Some(_deleted_line) = deleted_line {
                let next_line = self.text.remove(pos.row + 1);
                if let Some(current_line) = self.text.get_mut(pos.row) {
                    current_line.extend_from_line(next_line);
                    return true;
                }
            }
        }

        false
    }

    fn delete_char_at_internal(&mut self, pos: TextPosition) -> bool {
        if let Some(line) = self.text.get_mut(pos.row) {
            line.delete_char_at(pos.col)
        } else {
            false
        }
    }

    /// Deletes the character before `pos`.
    ///
    /// At the start of a line, this line is joined onto the previous one.
    /// Returns the position the cursor should move to, or `None` if there was
    /// nothing to delete.
    pub fn delete_char_before(&mut self, pos: TextPosition) -> Option<TextPosition> {
        if pos.col > 0 {
            // Find the character boundary before current position
            if let Some(line) = self.text.get(pos.row) {
                let char_pos = line.find_char_before(pos.col);
                if let Some(_ch) = line.char_at_col(char_pos)
                    && self.delete_char_at_internal(TextPosition {
                        row: pos.row,
                        col: char_pos,
                    })
                {
                    return Some(TextPosition {
                        row: pos.row,
                        col: char_pos,
                    });
                }
            }
        } else if pos.row > 0 {
            // Delete newline - merge with previous line
            let current_line = self.text.get(pos.row).cloned();
            if let Some(current_line) = current_line {
                let prev_row = pos.row - 1;
                let prev_col = self.cols(prev_row);

                self.text.remove(pos.row);
                if let Some(prev_line) = self.text.get_mut(prev_row) {
                    prev_line.extend_from_line(current_line);
                    return Some(TextPosition {
                        row: prev_row,
                        col: prev_col,
                    });
                }
            }
        }
        None
    }

    /// Inserts `ch` at `pos`, padding with empty lines if `pos` is past the end.
    ///
    /// Returns the position just after the inserted character, which is where
    /// the cursor should land.
    pub fn insert_char_at(&mut self, pos: TextPosition, ch: char) -> TextPosition {
        self.insert_char_at_internal(pos, ch)
    }

    fn insert_char_at_internal(&mut self, pos: TextPosition, ch: char) -> TextPosition {
        // Ensure we have enough rows
        while pos.row >= self.text.len() {
            self.text.push(TextLine::default());
        }

        if let Some(line) = self.text.get_mut(pos.row) {
            line.insert_char_at(pos.col, ch);

            // Return new cursor position
            TextPosition {
                row: pos.row,
                col: pos.col + crate::terminal::char_cols(ch),
            }
        } else {
            pos
        }
    }

    /// Returns the display column where the `char_index`-th character of `row`
    /// starts, or `None` if there is no such row.
    pub fn col_at_char_index(&self, row: usize, char_index: usize) -> Option<usize> {
        self.text
            .get(row)
            .map(|line| line.col_at_char_index(char_index))
    }

    /// Returns the index of the first character of `row` at or past column
    /// `col`, or `None` if there is no such row.
    pub fn char_index_at_col(&self, row: usize, col: usize) -> Option<usize> {
        self.line(row).map(|line| line.char_index_at_col(col))
    }

    /// Splits the line at `pos`, inserting a new line after it.
    ///
    /// Returns the position of the start of the new line, which is where the
    /// cursor should land.
    pub fn insert_newline_at(&mut self, pos: TextPosition) -> TextPosition {
        self.insert_newline_at_internal(pos)
    }

    fn insert_newline_at_internal(&mut self, pos: TextPosition) -> TextPosition {
        let current_row = pos.row;
        let current_col = pos.col;

        // Ensure we have enough rows
        while current_row >= self.text.len() {
            self.text.push(TextLine::default());
        }

        if let Some(current_line) = self.text.get_mut(current_row) {
            // Split the current line at cursor position
            let chars_after_cursor = current_line.split_off_at_col(current_col);

            // Create new line with the characters after cursor
            let new_line = TextLine::from_chars(chars_after_cursor);

            // Insert the new line after current line
            self.text.insert(current_row + 1, new_line);

            // Return new cursor position
            TextPosition {
                row: current_row + 1,
                col: 0,
            }
        } else {
            pos
        }
    }
}

/// One line of the buffer, held as characters rather than bytes.
///
/// Columns are display cells, so a wide character occupies more than one of
/// them; methods that take a column adjust for that as described per method.
///
/// The characters live behind an [`Arc`], so cloning a line -- and with it a
/// whole [`TextBuffer`](crate::TextBuffer) snapshot for the undo history -- only
/// bumps a reference count. A line that is then edited copies its characters
/// once, which keeps a snapshot's cost proportional to the lines an edit
/// actually touched rather than to the size of the file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(Arc<Vec<char>>);

impl TextLine {
    /// Builds a line from `chars`.
    fn from_chars(chars: Vec<char>) -> Self {
        TextLine(Arc::new(chars))
    }

    /// Appends every character of `other` to this line.
    fn extend_from_line(&mut self, other: TextLine) {
        Arc::make_mut(&mut self.0).extend_from_slice(&other.0);
    }

    /// Appends every character of `chars` to this line.
    fn extend_from_chars(&mut self, chars: Vec<char>) {
        Arc::make_mut(&mut self.0).extend(chars);
    }

    /// Keeps only the characters before column `col`, dropping the rest.
    ///
    /// The cut is made at the character whose start column is at or past `col`,
    /// so a column inside a wide character cuts before it.
    fn truncate_to_col(&mut self, col: usize) {
        let char_index = self.char_index_at_col(col);
        Arc::make_mut(&mut self.0).truncate(char_index);
    }

    /// Removes the characters in the column range `[start_col, end_col)`.
    ///
    /// The range is cut at character boundaries as the character whose start
    /// column is at or past a column.
    fn remove_cols(&mut self, start_col: usize, end_col: usize) {
        let start_index = self.char_index_at_col(start_col);
        let end_index = self.char_index_at_col(end_col);
        Arc::make_mut(&mut self.0).drain(start_index..end_index);
    }

    /// Removes and returns the characters at or past column `col`.
    ///
    /// The cut is made at the character whose start column is at or past `col`,
    /// so a column inside a wide character cuts before it.
    fn split_off_at_col(&mut self, col: usize) -> Vec<char> {
        let char_index = self.char_index_at_col(col);
        Arc::make_mut(&mut self.0).split_off(char_index)
    }

    /// Returns each character paired with the display column it starts at.
    pub fn char_cols(&self) -> impl Iterator<Item = (usize, char)> {
        let mut col = 0;
        self.0.iter().map(move |&ch| {
            let current_col = col;
            col += crate::terminal::char_cols(ch);
            (current_col, ch)
        })
    }

    /// Returns the character starting at column `col`, if one does.
    ///
    /// A column that falls inside a wide character does not match it.
    pub fn char_at_col(&self, col: usize) -> Option<char> {
        let mut current_col = 0;
        for &ch in self.0.iter() {
            if current_col == col {
                return Some(ch);
            }
            current_col += crate::terminal::char_cols(ch);
            if current_col > col {
                break;
            }
        }
        None
    }

    fn cols(&self) -> usize {
        self.0.iter().copied().map(crate::terminal::char_cols).sum()
    }

    fn adjust_to_char_boundary(&self, col: usize, floor: bool) -> usize {
        let mut start = 0;
        for &ch in self.0.iter() {
            let end = start + crate::terminal::char_cols(ch);
            if start == col {
                return col;
            } else if col < end {
                return if floor { start } else { end };
            }
            start = end;
        }
        start
    }

    fn delete_char_at(&mut self, col: usize) -> bool {
        let mut current_col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if current_col == col {
                Arc::make_mut(&mut self.0).remove(i);
                return true;
            }
            current_col += crate::terminal::char_cols(ch);
            if current_col > col {
                break;
            }
        }
        false
    }

    fn find_char_before(&self, col: usize) -> usize {
        let mut current_col = 0;
        for &ch in self.0.iter() {
            let next_col = current_col + crate::terminal::char_cols(ch);
            if next_col >= col {
                return current_col;
            }
            current_col = next_col;
        }
        current_col
    }

    fn insert_char_at(&mut self, col: usize, ch: char) {
        let mut char_index = 0;
        let mut current_col = 0;

        for (i, &existing_ch) in self.0.iter().enumerate() {
            if current_col >= col {
                char_index = i;
                break;
            }
            current_col += crate::terminal::char_cols(existing_ch);
            char_index = i + 1;
        }

        Arc::make_mut(&mut self.0).insert(char_index, ch);
    }

    /// Returns the number of characters in this line.
    pub fn char_count(&self) -> usize {
        self.0.len()
    }

    /// Returns the index of the first character starting at or past column
    /// `col`, or the character count when `col` is past the end.
    fn char_index_at_col(&self, col: usize) -> usize {
        let mut current_col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if current_col >= col {
                return i;
            }
            current_col += crate::terminal::char_cols(ch);
        }
        self.0.len()
    }

    /// Returns the display column where the `char_index`-th character starts,
    /// or the line's width when `char_index` is past the end.
    fn col_at_char_index(&self, char_index: usize) -> usize {
        let mut col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if i >= char_index {
                return col;
            }
            col += crate::terminal::char_cols(ch);
        }
        col
    }
}

impl std::fmt::Display for TextLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &ch in self.0.iter() {
            std::fmt::Write::write_char(f, ch)?;
        }
        Ok(())
    }
}

/// A position in a [`TextBuffer`].
///
/// Both fields are 0-based. `col` counts display cells, not characters, so a
/// position must sit on a character boundary to name a character.
///
/// This names a spot in the buffer, so it is not a
/// [`tuinix::Position`](::tuinix::Position), which names a spot on the screen;
/// the two differ by the viewport's offset.
///
/// The ordering is by `row` first and then `col`, so a range of positions can
/// be compared directly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    /// The line, 0-based.
    pub row: usize,

    /// The display column, 0-based.
    pub col: usize,
}
