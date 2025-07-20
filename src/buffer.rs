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
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(pub Vec<char>);

impl TextLine {
    pub fn cols(&self) -> usize {
        self.0.iter().filter_map(|c| c.width()).sum()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub row: usize,
    pub col: usize,
}
