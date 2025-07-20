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
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(pub Vec<char>);

impl TextLine {
    fn cols(&self) -> usize {
        self.0.iter().filter_map(|c| c.width()).sum()
    }

    fn adjust_to_char_boundary(&self, col: usize, floor: bool) -> usize {
        let mut start = 0;
        for ch in &self.0 {
            let end = start + ch.width().unwrap_or_default();
            if start == col {
                return col;
            } else if end < col {
                start = end;
            } else if floor {
                return start;
            } else {
                return end;
            }
        }
        start
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub row: usize,
    pub col: usize,
}
