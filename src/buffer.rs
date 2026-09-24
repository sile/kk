//! The buffer model: lines of characters, with column-aware edits.

/// The text being edited.
///
/// A buffer is a list of [`TextLine`]s held in memory. Every edit marks it
/// [`dirty`](TextBuffer::dirty), and [`mark_saved`](TextBuffer::mark_saved)
/// clears that flag once the text has been persisted.
///
/// Rows and columns here are 0-based. A column counts display cells, not
/// characters, so a column must be adjusted to a character boundary before it
/// can address one of the line's characters.
#[derive(Debug, Default, Clone)]
pub struct TextBuffer {
    /// The lines, in order.
    pub text: Vec<TextLine>,

    /// Whether the buffer has edits that have not been saved.
    pub dirty: bool,
}

impl TextBuffer {
    /// Builds a buffer from `text`, splitting it into lines.
    ///
    /// The result starts out clean. A trailing newline does not produce a
    /// final empty line.
    pub fn from_text(text: &str) -> Self {
        Self {
            text: text
                .lines()
                .map(|l| TextLine(l.chars().collect()))
                .collect(),
            dirty: false,
        }
    }

    /// Replaces the contents with `text`, as if the file had been reloaded.
    pub fn replace_from_text(&mut self, text: &str) {
        self.text = text
            .lines()
            .map(|l| TextLine(l.chars().collect()))
            .collect();
        self.dirty = false;
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

    /// Marks the buffer as saved.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Returns the number of lines.
    pub fn rows(&self) -> usize {
        self.text.len()
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
            self.dirty = true;
            return true;
        }

        // Handle forward delete at line end (merge with next line)
        if pos.col >= self.cols(pos.row) && pos.row < self.text.len().saturating_sub(1) {
            let deleted_line = self.text.get(pos.row + 1).cloned();
            if let Some(_deleted_line) = deleted_line {
                let next_line = self.text.remove(pos.row + 1);
                if let Some(current_line) = self.text.get_mut(pos.row) {
                    current_line.extend_from_line(next_line);
                    self.dirty = true;
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
                    self.dirty = true;
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
                    self.dirty = true;
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
        let new_pos = self.insert_char_at_internal(pos, ch);
        self.dirty = true;
        new_pos
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
        if let Some(line) = self.text.get(row) {
            let mut current_col = 0;
            for (i, &ch) in line.0.iter().enumerate() {
                if current_col >= col {
                    return Some(i);
                }
                current_col += crate::terminal::char_cols(ch);
            }
            Some(line.0.len())
        } else {
            None
        }
    }

    /// Splits the line at `pos`, inserting a new line after it.
    ///
    /// Returns the position of the start of the new line, which is where the
    /// cursor should land.
    pub fn insert_newline_at(&mut self, pos: TextPosition) -> TextPosition {
        let new_pos = self.insert_newline_at_internal(pos);
        self.dirty = true;
        new_pos
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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(pub Vec<char>);

impl TextLine {
    /// Builds a line from `chars`.
    pub fn from_chars(chars: Vec<char>) -> Self {
        TextLine(chars)
    }

    /// Appends every character of `other` to this line.
    pub fn extend_from_line(&mut self, other: TextLine) {
        self.0.extend(other.0);
    }

    /// Removes and returns the characters at or past column `col`.
    ///
    /// The cut is made at the character whose start column is at or past `col`,
    /// so a column inside a wide character cuts before it.
    pub fn split_off_at_col(&mut self, col: usize) -> Vec<char> {
        let char_index = self.char_index_at_col(col);
        self.0.split_off(char_index)
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
        for &ch in &self.0 {
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
        for &ch in &self.0 {
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
                self.0.remove(i);
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
        for &ch in &self.0 {
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

        self.0.insert(char_index, ch);
    }

    /// Returns the index of the first character starting at or past column
    /// `col`, or the character count when `col` is past the end.
    pub fn char_index_at_col(&self, col: usize) -> usize {
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
    pub fn col_at_char_index(&self, char_index: usize) -> usize {
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
        for &ch in &self.0 {
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
/// The ordering is by `row` first and then `col`, so a range of positions can
/// be compared directly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    /// The line, 0-based.
    pub row: usize,

    /// The display column, 0-based.
    pub col: usize,
}
