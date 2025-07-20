use std::path::PathBuf;

use orfail::OrFail;
use tuinix::{KeyCode, TerminalPosition, TerminalSize};

use crate::{
    buffer::{TextBuffer, TextPosition},
    keybindings::KeybindingsContext,
};

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub cursor: TextPosition,
    pub viewport: TextPosition, // Top-left position of the visible text area
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
            viewport: TextPosition::default(),
            buffer,
            message: None,
            context: KeybindingsContext::default(),
        })
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub fn terminal_cursor_position(&self) -> TerminalPosition {
        let pos = self.cursor_position();
        let screen_row = pos.row.saturating_sub(self.viewport.row);
        let screen_col = pos.col.saturating_sub(self.viewport.col);
        TerminalPosition::row_col(screen_row, screen_col)
    }

    pub fn cursor_position(&self) -> TextPosition {
        self.buffer.adjust_to_char_boundary(self.cursor, true)
    }

    pub fn adjust_viewport(&mut self, text_area_size: TerminalSize) {
        let cursor_pos = self.cursor_position();
        let available_rows = text_area_size.rows;
        let available_cols = text_area_size.cols;

        // Adjust vertical viewport
        if cursor_pos.row < self.viewport.row {
            // Cursor is above viewport, scroll up
            self.viewport.row = cursor_pos.row;
        } else if cursor_pos.row >= self.viewport.row + available_rows {
            // Cursor is below viewport, scroll down
            self.viewport.row = cursor_pos
                .row
                .saturating_sub(available_rows.saturating_sub(1));
        }

        // Adjust horizontal viewport
        if cursor_pos.col < self.viewport.col {
            // Cursor is left of viewport, scroll left
            self.viewport.col = cursor_pos.col;
        } else if cursor_pos.col >= self.viewport.col + available_cols {
            // Cursor is right of viewport, scroll right
            self.viewport.col = cursor_pos
                .col
                .saturating_sub(available_cols.saturating_sub(1));
        }
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

    pub fn handle_cursor_buffer_start(&mut self) {
        self.cursor = TextPosition::default();
    }

    pub fn handle_cursor_buffer_end(&mut self) {
        self.cursor.row = self.buffer.rows();
        self.cursor.col = 0;
    }

    pub fn handle_char_delete_backward(&mut self) {
        if let Some(new_pos) = self.buffer.delete_char_before(self.cursor) {
            self.cursor = new_pos;
        }
    }

    pub fn handle_char_delete_forward(&mut self) {
        self.buffer.delete_char_at(self.cursor);
    }

    pub fn handle_buffer_save(&mut self) -> orfail::Result<()> {
        self.buffer.save_to_file(&self.path).or_fail()?;
        self.set_message(format!("Saved: {}", self.path.display()));
        Ok(())
    }

    pub fn handle_buffer_reload(&mut self) -> orfail::Result<()> {
        // Reload the buffer from file
        self.buffer.load_file(&self.path).or_fail()?;

        // Try to preserve cursor position, but adjust if the file has changed
        let max_row = self.buffer.rows();
        self.cursor.row = self.cursor.row.min(max_row);

        if self.cursor.row < max_row {
            let max_col = self.buffer.cols(self.cursor.row);
            self.cursor.col = self.cursor.col.min(max_col);
        } else {
            self.cursor.col = 0;
        }

        // Adjust cursor to proper character boundary
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);

        self.set_message(format!("Reloaded: {}", self.path.display()));
        Ok(())
    }

    pub fn handle_char_insert(&mut self, key: tuinix::KeyInput) {
        // Only insert printable characters
        if let KeyCode::Char(ch) = key.code
            && !ch.is_control()
        {
            self.cursor = self.buffer.insert_char_at(self.cursor, ch);
        }
    }
}
