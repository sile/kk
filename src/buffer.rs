use std::path::Path;

use orfail::OrFail;

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
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(Vec<char>);
