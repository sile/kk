use std::path::Path;

use orfail::OrFail;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Default)]
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
        if let Some(line) = self.text.get_mut(pos.row) {
            if line.delete_char_at(pos.col) {
                self.dirty = true;
                return true;
            }
        }

        // If we couldn't delete a character on the current line,
        // try to merge with the next line (for forward delete at line end)
        if pos.col >= self.cols(pos.row) && pos.row < self.text.len().saturating_sub(1) {
            let next_line = self.text.remove(pos.row + 1);
            if let Some(current_line) = self.text.get_mut(pos.row) {
                current_line.0.extend(next_line.0);
                self.dirty = true;
                return true;
            }
        }

        false
    }

    pub fn delete_char_before(&mut self, pos: TextPosition) -> Option<TextPosition> {
        if pos.col > 0 {
            // Find the character boundary before current position
            if let Some(line) = self.text.get_mut(pos.row) {
                let char_pos = line.find_char_before(pos.col);
                if line.delete_char_at(char_pos) {
                    self.dirty = true;
                    return Some(TextPosition {
                        row: pos.row,
                        col: char_pos,
                    });
                }
            }
        } else if pos.row > 0 {
            // Delete newline - merge with previous line
            let current_line = self.text.remove(pos.row);
            let prev_row = pos.row - 1;
            let prev_col = self.cols(prev_row);

            if let Some(prev_line) = self.text.get_mut(prev_row) {
                prev_line.0.extend(current_line.0);
                self.dirty = true;
                return Some(TextPosition {
                    row: prev_row,
                    col: prev_col,
                });
            }
        }
        None
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(Vec<char>);

impl TextLine {
    pub fn char_cols(&self) -> impl Iterator<Item = (usize, char)> {
        let mut col = 0;
        self.0.iter().map(move |&ch| {
            let current_col = col;
            col += ch.width().unwrap_or_default();
            (current_col, ch)
        })
    }

    fn cols(&self) -> usize {
        self.0.iter().filter_map(|c| c.width()).sum()
    }

    fn adjust_to_char_boundary(&self, col: usize, floor: bool) -> usize {
        let mut start = 0;
        for ch in &self.0 {
            let end = start + ch.width().unwrap_or_default();
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
            current_col += ch.width().unwrap_or_default();
            if current_col > col {
                break;
            }
        }
        false
    }

    fn find_char_before(&self, col: usize) -> usize {
        let mut current_col = 0;
        for &ch in &self.0 {
            let next_col = current_col + ch.width().unwrap_or_default();
            if next_col >= col {
                return current_col;
            }
            current_col = next_col;
        }
        current_col
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub row: usize,
    pub col: usize,
}
