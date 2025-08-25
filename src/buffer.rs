use std::path::Path;

use orfail::OrFail;

#[derive(Debug, Default, Clone)]
pub struct TextBuffer {
    pub text: Vec<TextLine>,
    pub dirty: bool,
}

impl TextBuffer {
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> orfail::Result<()> {
        let text = std::fs::read_to_string(&path)
            .or_fail_with(|e| format!("failed to read file {}: {e}", path.as_ref().display()))?;
        self.text = text
            .lines()
            .map(|l| TextLine(l.chars().collect()))
            .collect();
        self.dirty = false;
        Ok(())
    }

    pub fn to_single_text(&self) -> String {
        self.text
            .iter()
            .flat_map(|line| line.0.iter().copied().chain(std::iter::once('\n')))
            .collect()
    }

    pub fn rows(&self) -> usize {
        self.text.len()
    }

    pub fn cols(&self, row: usize) -> usize {
        self.text.get(row).map(|l| l.cols()).unwrap_or_default()
    }

    pub fn adjust_to_char_boundary(&self, mut pos: TextPosition, floor: bool) -> TextPosition {
        if let Some(line) = self.text.get(pos.row) {
            pos.col = line.adjust_to_char_boundary(pos.col, floor);
        } else {
            pos.col = 0;
        }
        pos
    }

    pub fn delete_char_at(&mut self, pos: TextPosition) -> bool {
        // Store the character for undo before deleting
        if let Some(line) = self.text.get(pos.row) {
            if let Some(_ch) = line.char_at_col(pos.col) {
                self.delete_char_at_internal(pos);
                self.dirty = true;
                return true;
            }
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

    pub fn delete_char_before(&mut self, pos: TextPosition) -> Option<TextPosition> {
        if pos.col > 0 {
            // Find the character boundary before current position
            if let Some(line) = self.text.get(pos.row) {
                let char_pos = line.find_char_before(pos.col);
                if let Some(_ch) = line.char_at_col(char_pos) {
                    if self.delete_char_at_internal(TextPosition {
                        row: pos.row,
                        col: char_pos,
                    }) {
                        self.dirty = true;
                        return Some(TextPosition {
                            row: pos.row,
                            col: char_pos,
                        });
                    }
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

    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> orfail::Result<()> {
        let mut content = self
            .text
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        content.push('\n');

        std::fs::write(&path, content)
            .or_fail_with(|e| format!("failed to write file {}: {e}", path.as_ref().display()))?;

        self.dirty = false;
        Ok(())
    }

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
                col: pos.col + mame::terminal::char_cols(ch),
            }
        } else {
            pos
        }
    }

    pub fn col_at_char_index(&self, row: usize, char_index: usize) -> Option<usize> {
        if let Some(line) = self.text.get(row) {
            Some(line.col_at_char_index(char_index))
        } else {
            None
        }
    }

    pub fn char_index_at_col(&self, row: usize, col: usize) -> Option<usize> {
        if let Some(line) = self.text.get(row) {
            let mut current_col = 0;
            for (i, &ch) in line.0.iter().enumerate() {
                if current_col >= col {
                    return Some(i);
                }
                current_col += mame::terminal::char_cols(ch);
            }
            Some(line.0.len())
        } else {
            None
        }
    }

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(pub Vec<char>);

impl TextLine {
    pub fn from_chars(chars: Vec<char>) -> Self {
        TextLine(chars)
    }

    pub fn to_string(&self) -> String {
        self.0.iter().collect()
    }

    pub fn extend_from_line(&mut self, other: TextLine) {
        self.0.extend(other.0);
    }

    pub fn split_off_at_col(&mut self, col: usize) -> Vec<char> {
        let char_index = self.char_index_at_col(col);
        self.0.split_off(char_index)
    }

    pub fn char_cols(&self) -> impl Iterator<Item = (usize, char)> {
        let mut col = 0;
        self.0.iter().map(move |&ch| {
            let current_col = col;
            col += mame::terminal::char_cols(ch);
            (current_col, ch)
        })
    }

    pub fn char_at_col(&self, col: usize) -> Option<char> {
        let mut current_col = 0;
        for &ch in &self.0 {
            if current_col == col {
                return Some(ch);
            }
            current_col += mame::terminal::char_cols(ch);
            if current_col > col {
                break;
            }
        }
        None
    }

    fn cols(&self) -> usize {
        self.0.iter().copied().map(mame::terminal::char_cols).sum()
    }

    fn adjust_to_char_boundary(&self, col: usize, floor: bool) -> usize {
        let mut start = 0;
        for &ch in &self.0 {
            let end = start + mame::terminal::char_cols(ch);
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
            current_col += mame::terminal::char_cols(ch);
            if current_col > col {
                break;
            }
        }
        false
    }

    fn find_char_before(&self, col: usize) -> usize {
        let mut current_col = 0;
        for &ch in &self.0 {
            let next_col = current_col + mame::terminal::char_cols(ch);
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
            current_col += mame::terminal::char_cols(existing_ch);
            char_index = i + 1;
        }

        self.0.insert(char_index, ch);
    }

    pub fn char_index_at_col(&self, col: usize) -> usize {
        let mut current_col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if current_col >= col {
                return i;
            }
            current_col += mame::terminal::char_cols(ch);
        }
        self.0.len()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn col_at_char_index(&self, char_index: usize) -> usize {
        let mut col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if i >= char_index {
                return col;
            }
            col += mame::terminal::char_cols(ch);
        }
        col
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub row: usize, // 0 origin
    pub col: usize, // 0 origin
}
