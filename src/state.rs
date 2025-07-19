use std::path::PathBuf;

use orfail::OrFail;
use tuinix::TerminalPosition;

use crate::{
    buffer::{TextBuffer, TextPosition},
    config::Config,
    keybindings::KeybindingsContext,
};

#[derive(Debug)]
pub struct State {
    pub config: Config,
    pub path: PathBuf,
    pub cursor: TextPosition,
    pub buffer: TextBuffer,
    pub message: Option<String>,
    pub keybindings_context: KeybindingsContext,
}

impl State {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let mut buffer = TextBuffer::default();
        buffer.load_file(&path).or_fail()?;
        Ok(Self {
            config: Config::default(),
            path,
            cursor: TextPosition::default(),
            buffer,
            message: None,
            keybindings_context: KeybindingsContext::default(),
        })
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub fn terminal_cursor_position(&self) -> TerminalPosition {
        TerminalPosition::row_col(self.cursor.row, self.cursor.col)
    }
}
