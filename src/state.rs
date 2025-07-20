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
        let pos = self.buffer.adjust_to_char_boundary(self.cursor, true);
        TerminalPosition::row_col(pos.row, pos.col)
    }

    pub fn handle_move_action(&mut self, MoveAction { rows, cols }: MoveAction) {
        // Calculate new row position
        self.cursor.row = self
            .cursor
            .row
            .saturating_add_signed(rows)
            .min(self.buffer.rows());
        if cols == 0 {
            return;
        }

        // Calculate new column position
        self.cursor.col = self
            .cursor
            .col
            .saturating_add_signed(cols)
            .min(self.buffer.cols(self.cursor.row));
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, cols < 0);
    }
}
