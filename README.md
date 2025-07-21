# kk

Features
-------

- Exciplit
  - All actions are trigerred by users, no background tasks, no implicit keybindings
- not framework or environment, just an text editor

Intendedly Unsupported Features
-------------------------------

- Color
- Plugin (or extension) system
- Multi (split) windows
- Multi buffers

Example Text (for dev)
-----------------------

This is looooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooong line.

```
use std::path::PathBuf;

use orfail::OrFail;
use tuinix::TerminalPosition;

use crate::{
    buffer::{TextBuffer, TextPosition},
    keybindings::KeybindingsContext,
};

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub cursor: TextPosition,
    pub buffer: TextBuffer,
    pub message: Option<String>,
    pub context: KeybindingsContext,
}

impl State {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let mut buffer = TextBuffer::default();
        buffer.load_file(&path).or_fail()?;
        Ok(Self {
            path,
            cursor: TextPosition::default(),
            buffer,
            message: None,
            context: KeybindingsContext::default(),
        })
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub fn terminal_cursor_position(&self) -> TerminalPosition {
        let pos = self.buffer.adjust_to_char_boundary(self.cursor, true);
        TerminalPosition::row_col(pos.row, pos.col)
    }

    pub fn handle_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(1);
    }

    pub fn handle_cursor_down(&mut self) {
        self.cursor.row = self.cursor.row.saturating_add(1).min(self.buffer.rows());
    }

    pub fn handle_cursor_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col = self.cursor.col.saturating_sub(1);
            self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        } else if self.cursor.row > 0 {
            // Move to end of previous line
            self.cursor.row = self.cursor.row.saturating_sub(1);
            self.cursor.col = self.buffer.cols(self.cursor.row);
        }
    }

    pub fn handle_cursor_right(&mut self) {
        let current_cols = self.buffer.cols(self.cursor.row);
        if self.cursor.col < current_cols {
            self.cursor.col = self.cursor.col.saturating_add(1);
            self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, false);
        } else if self.cursor.row < self.buffer.rows() {
            // Move to beginning of next line
            self.cursor.row = self.cursor.row.saturating_add(1);
            self.cursor.col = 0;
        }
    }

    pub fn handle_cursor_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn handle_cursor_line_end(&mut self) {
        self.cursor.col = self.buffer.cols(self.cursor.row);
    }
}
```
