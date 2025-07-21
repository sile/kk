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
    pub mark: Option<TextPosition>,
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
            mark: None,
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

    pub fn handle_newline_insert(&mut self) {
        self.cursor = self.buffer.insert_newline_at(self.cursor);
    }

    pub fn handle_buffer_undo(&mut self) {
        if let Some(new_cursor) = self.buffer.undo() {
            self.cursor = new_cursor;
            self.set_message("Undo");
        } else {
            self.set_message("Nothing to undo");
        }
    }

    pub fn handle_mark_set(&mut self) {
        let cursor_pos = self.cursor_position();
        if self.mark == Some(cursor_pos) {
            // If mark is already at cursor position, deactivate it
            self.mark = None;
            self.set_message("Mark deactivated");
        } else {
            // Set mark at current cursor position
            self.mark = Some(cursor_pos);
            self.set_message("Mark set");
        }
    }

    pub fn handle_mark_copy(&mut self) {
        if let Some(mark_pos) = self.mark.take() {
            let cursor_pos = self.cursor_position();
            let (start, end) = if mark_pos <= cursor_pos {
                (mark_pos, cursor_pos)
            } else {
                (cursor_pos, mark_pos)
            };

            if let Some(text) = self.get_text_in_range(start, end) {
                // TODO: Implement clipboard functionality
                self.set_message(format!("Copied {} characters", text.len()));
            } else {
                self.set_message("Nothing to copy");
            }
        } else {
            self.set_message("No mark set");
        }
    }

    pub fn handle_mark_cut(&mut self) {
        if let Some(mark_pos) = self.mark.take() {
            let cursor_pos = self.cursor_position();
            let (start, end) = if mark_pos <= cursor_pos {
                (mark_pos, cursor_pos)
            } else {
                (cursor_pos, mark_pos)
            };

            if let Some(text) = self.get_text_in_range(start, end) {
                // Delete the selected text
                self.delete_text_in_range(start, end);
                self.cursor = start;
                self.mark = None;

                // TODO: Implement clipboard functionality
                self.set_message(format!("Cut {} characters", text.len()));
            } else {
                self.set_message("Nothing to cut");
            }
        } else {
            self.set_message("No mark set");
        }
    }

    // Helper method to get text in a range
    fn get_text_in_range(&self, start: TextPosition, end: TextPosition) -> Option<String> {
        if start == end {
            return None;
        }

        let mut result = String::new();

        if start.row == end.row {
            // Single line selection
            if let Some(line) = self.buffer.text.get(start.row) {
                for (col, ch) in line.char_cols() {
                    if col >= start.col && col < end.col {
                        result.push(ch);
                    }
                }
            }
        } else {
            // Multi-line selection
            for row in start.row..=end.row {
                if let Some(line) = self.buffer.text.get(row) {
                    if row == start.row {
                        // First line: from start.col to end of line
                        for (col, ch) in line.char_cols() {
                            if col >= start.col {
                                result.push(ch);
                            }
                        }
                        result.push('\n');
                    } else if row == end.row {
                        // Last line: from start of line to end.col
                        for (col, ch) in line.char_cols() {
                            if col < end.col {
                                result.push(ch);
                            }
                        }
                    } else {
                        // Middle lines: entire line
                        result.push_str(&line.to_string());
                        result.push('\n');
                    }
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    // Helper method to delete text in a range
    fn delete_text_in_range(&mut self, start: TextPosition, end: TextPosition) {
        if start == end {
            return;
        }

        // TODO: This should be implemented as a compound undo action
        // For now, we'll do a simple implementation

        if start.row == end.row {
            // Single line deletion
            if let Some(line) = self.buffer.text.get_mut(start.row) {
                let start_char_idx = line.char_index_at_col(start.col);
                let end_char_idx = line.char_index_at_col(end.col);

                for _ in start_char_idx..end_char_idx {
                    if start_char_idx < line.len() {
                        line.0.remove(start_char_idx);
                    }
                }
            }
        } else {
            // Multi-line deletion
            // Remove complete middle lines
            for _ in start.row + 1..end.row {
                if start.row + 1 < self.buffer.text.len() {
                    self.buffer.text.remove(start.row + 1);
                }
            }

            // Handle first and last lines
            if let Some(start_line) = self.buffer.text.get_mut(start.row) {
                let chars_to_keep: Vec<char> = start_line
                    .char_cols()
                    .filter(|(col, _)| *col < start.col)
                    .map(|(_, ch)| ch)
                    .collect();
                start_line.0 = chars_to_keep;
            }

            if start.row + 1 < self.buffer.text.len() {
                if let Some(end_line) = self.buffer.text.get(start.row + 1).cloned() {
                    let chars_to_keep: Vec<char> = end_line
                        .char_cols()
                        .filter(|(col, _)| *col >= end.col)
                        .map(|(_, ch)| ch)
                        .collect();

                    if let Some(start_line) = self.buffer.text.get_mut(start.row) {
                        start_line.0.extend(chars_to_keep);
                    }

                    self.buffer.text.remove(start.row + 1);
                }
            }
        }

        self.buffer.dirty = true;
    }
}
