use std::path::PathBuf;

use orfail::OrFail;
use tuinix::TerminalPosition;

use crate::{
    action::MoveAction,
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
        TerminalPosition::row_col(self.cursor.row, self.cursor.col)
    }

    pub fn handle_move_action(&mut self, MoveAction { rows, cols }: MoveAction) {
        // Calculate new row position
        let new_row = if rows >= 0 {
            (self.cursor.row + rows as usize).min(self.buffer.rows().saturating_sub(1))
        } else {
            self.cursor.row.saturating_sub((-rows) as usize)
        };

        // Calculate new column position
        let max_cols = self.buffer.cols(new_row);
        let new_col = if cols >= 0 {
            (self.cursor.col + cols as usize).min(max_cols)
        } else {
            self.cursor.col.saturating_sub((-cols) as usize)
        };

        // Update cursor position
        self.cursor = TextPosition {
            row: new_row,
            col: new_col,
        };
    }
}
