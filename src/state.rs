use std::{collections::VecDeque, num::NonZeroUsize, path::PathBuf};

use orfail::OrFail;
use tuinix::{KeyCode, TerminalPosition, TerminalSize};

use crate::{
    action::ExternalCommandAction,
    anchor::CursorAnchor,
    buffer::{TextBuffer, TextPosition},
    clipboard::Clipboard,
    keybindings::KeybindingsContext,
};

pub const MAX_HISTORY_SIZE: usize = 1000;

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub cursor: TextPosition,
    pub viewport: TextPosition, // Top-left position of the visible text area
    pub recenter_viewport: bool,
    pub buffer: TextBuffer,
    pub message: Option<String>,
    pub context: KeybindingsContext,
    pub mark: Option<TextPosition>,
    pub clipboard: Clipboard,
    pub editing: bool,
    pub history: VecDeque<(TextPosition, TextBuffer)>,
    pub undo_index: usize,
}

impl State {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let mut buffer = TextBuffer::default();
        buffer.load_file(&path).or_fail()?;
        Ok(Self {
            path,
            cursor: TextPosition::default(),
            viewport: TextPosition::default(),
            recenter_viewport: false,
            buffer,
            message: None,
            context: KeybindingsContext::default(),
            mark: None,
            clipboard: Clipboard::default(),
            editing: false,
            history: VecDeque::new(),
            undo_index: 0,
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

        if self.recenter_viewport {
            // Center the cursor in the viewport
            self.viewport.row = cursor_pos.row.saturating_sub(available_rows / 2);
            self.viewport.col = cursor_pos.col.saturating_sub(available_cols / 2);
            self.recenter_viewport = false;
            return;
        }

        // Existing viewport adjustment logic
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

    fn start_editing(&mut self) {
        if self.editing {
            return;
        }

        while self.history.len() >= MAX_HISTORY_SIZE {
            self.history.pop_front();
        }

        self.history.push_back((self.cursor, self.buffer.clone()));
        self.undo_index = self.history.len();

        self.editing = true;
    }

    fn finish_editing(&mut self) {
        self.editing = false;
    }

    pub fn handle_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(1);
        self.finish_editing();
    }

    pub fn handle_cursor_down(&mut self) {
        self.cursor.row = self.cursor.row.saturating_add(1).min(self.buffer.rows());
        self.finish_editing();
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
        self.finish_editing();
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
        self.finish_editing();
    }

    pub fn handle_cursor_line_start(&mut self) {
        self.cursor.col = 0;
        self.finish_editing();
    }

    pub fn handle_cursor_line_end(&mut self) {
        self.cursor.col = self.buffer.cols(self.cursor.row);
        self.finish_editing();
    }

    pub fn handle_cursor_buffer_start(&mut self) {
        self.cursor = TextPosition::default();
        self.finish_editing();
    }

    pub fn handle_cursor_buffer_end(&mut self) {
        self.cursor.row = self.buffer.rows();
        self.cursor.col = 0;
        self.finish_editing();
    }

    pub fn handle_char_delete_backward(&mut self) {
        self.start_editing();
        if let Some(new_pos) = self.buffer.delete_char_before(self.cursor) {
            self.cursor = new_pos;
        }
    }

    pub fn handle_char_delete_forward(&mut self) {
        self.start_editing();
        self.buffer.delete_char_at(self.cursor);
    }

    pub fn handle_buffer_save(&mut self) -> orfail::Result<()> {
        self.buffer.save_to_file(&self.path).or_fail()?;
        self.set_message(format!("Saved: {}", self.path.display()));
        Ok(())
    }

    pub fn handle_buffer_reload(&mut self) -> orfail::Result<()> {
        self.finish_editing();
        self.start_editing();

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
        self.finish_editing();
        Ok(())
    }

    pub fn handle_char_insert(&mut self, key: tuinix::KeyInput) {
        self.start_editing();
        // Only insert printable characters
        if let KeyCode::Char(ch) = key.code
            && !ch.is_control()
        {
            self.cursor = self.buffer.insert_char_at(self.cursor, ch);
        }
    }

    pub fn handle_newline_insert(&mut self) {
        self.finish_editing();
        self.start_editing();
        self.cursor = self.buffer.insert_newline_at(self.cursor);
        self.finish_editing();
    }

    pub fn handle_buffer_undo(&mut self) {
        if self.editing {
            self.finish_editing();
            self.start_editing();
            self.editing = false;
        }

        let Some(i) = self.undo_index.checked_sub(1) else {
            self.set_message("Nothing to undo");
            return;
        };

        let (cursor, buffer) = self.history[i].clone();
        self.cursor = cursor;
        self.buffer = buffer;
        self.undo_index = i;
        self.set_message(format!("Undo ({})", self.history.len() - i));
    }

    pub fn handle_mark_set(&mut self) {
        self.finish_editing();

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

    pub fn handle_mark_copy(&mut self) -> orfail::Result<()> {
        self.finish_editing();

        if let Some(mark_pos) = self.mark.take() {
            let cursor_pos = self.cursor_position();
            let (start, end) = if mark_pos <= cursor_pos {
                (mark_pos, cursor_pos)
            } else {
                (cursor_pos, mark_pos)
            };

            if let Some(text) = self.get_text_in_range(start, end) {
                self.clipboard.write(&text).or_fail()?;
                self.set_message(format!("Copied {} characters", text.len()));
            } else {
                self.set_message("Nothing to copy");
            }
        } else {
            self.set_message("No mark set");
        }
        Ok(())
    }

    pub fn handle_mark_cut(&mut self) -> orfail::Result<()> {
        self.finish_editing();

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

                self.clipboard.write(&text).or_fail()?;
                self.set_message(format!("Cut {} characters", text.len()));
            } else {
                self.set_message("Nothing to cut");
            }
        } else {
            self.set_message("No mark set");
        }
        Ok(())
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

    pub fn handle_clipboard_paste(&mut self) -> orfail::Result<()> {
        self.finish_editing();

        let text = self.clipboard.read().or_fail()?;

        if text.is_empty() {
            self.set_message("Clipboard is empty");
            return Ok(());
        }

        // Split text into lines
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            self.set_message("Nothing to paste");
            return Ok(());
        }
        self.start_editing();

        // Insert the text
        if lines.len() == 1 {
            // Single line paste
            let line = lines[0];
            for ch in line.chars() {
                self.cursor = self.buffer.insert_char_at(self.cursor, ch);
            }
            self.set_message(format!("Pasted {} characters", line.len()));
        } else {
            // Multi-line paste
            let mut total_chars = 0;

            // Insert first line
            for ch in lines[0].chars() {
                self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                total_chars += 1;
            }

            // Insert newline and subsequent lines
            for line in &lines[1..] {
                self.cursor = self.buffer.insert_newline_at(self.cursor);
                total_chars += 1; // Count the newline

                for ch in line.chars() {
                    self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                    total_chars += 1;
                }
            }

            self.set_message(format!(
                "Pasted {} characters across {} lines",
                total_chars,
                lines.len()
            ));
        }

        self.finish_editing();
        Ok(())
    }

    pub fn handle_external_command(
        &mut self,
        action: &ExternalCommandAction,
    ) -> orfail::Result<()> {
        self.finish_editing();

        let mut cmd = std::process::Command::new(&action.command);

        for arg in &action.args {
            cmd.arg(arg);
        }

        let stdin_input = if let Some(mark_pos) = self.mark {
            let cursor_pos = self.cursor_position();
            let (start, end) = if mark_pos <= cursor_pos {
                (mark_pos, cursor_pos)
            } else {
                (cursor_pos, mark_pos)
            };
            self.get_text_in_range(start, end)
        } else {
            None
        };
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Err(e) => {
                self.set_message(format!("Failed to execute command: {}", e));
                return Ok(());
            }
            Ok(child) => child,
        };

        // Write to stdin if we have marked text
        if let Some(mut stdin) = child.stdin.take() {
            if let Some(text) = stdin_input {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
        }

        let output = match child.wait_with_output() {
            Err(e) => {
                self.set_message(format!("Failed to wait for command: {}", e));
                return Ok(());
            }
            Ok(output) => output,
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.set_message(format!("Command failed: {}", stderr.trim()));
            return Ok(());
        }

        self.start_editing();

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(mark_pos) = self.mark.take() {
            // Replace marked region with output
            let cursor_pos = self.cursor_position();
            let (start, end) = if mark_pos <= cursor_pos {
                (mark_pos, cursor_pos)
            } else {
                (cursor_pos, mark_pos)
            };

            self.start_editing();

            // Delete the marked region
            self.delete_text_in_range(start, end);
            self.cursor = start;

            // Insert the command output
            let output_str = stdout.trim_end(); // Remove trailing whitespace/newlines
            let lines: Vec<&str> = output_str.lines().collect();

            if !lines.is_empty() {
                // Insert first line
                for ch in lines[0].chars() {
                    self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                }

                // Insert subsequent lines with newlines
                for line in &lines[1..] {
                    self.cursor = self.buffer.insert_newline_at(self.cursor);
                    for ch in line.chars() {
                        self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                    }
                }
            }

            self.finish_editing();
            self.set_message(format!(
                "Replaced region with command output ({} chars)",
                output_str.len()
            ));
        } else {
            // No marked region, insert output at cursor
            self.start_editing();

            let output_str = stdout.trim_end();
            let lines: Vec<&str> = output_str.lines().collect();

            if !lines.is_empty() {
                // Insert first line
                for ch in lines[0].chars() {
                    self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                }

                // Insert subsequent lines with newlines
                for line in &lines[1..] {
                    self.cursor = self.buffer.insert_newline_at(self.cursor);
                    for ch in line.chars() {
                        self.cursor = self.buffer.insert_char_at(self.cursor, ch);
                    }
                }
            }

            self.finish_editing();
            self.set_message(format!(
                "Inserted command output ({} chars)",
                output_str.len()
            ));
        }
        self.finish_editing();

        Ok(())
    }

    pub fn current_cursor_anchor(&self) -> CursorAnchor {
        CursorAnchor {
            path: self.path.clone(),
            line: NonZeroUsize::MIN.saturating_add(self.cursor.row),
            char: NonZeroUsize::MIN.saturating_add(
                self.buffer
                    .char_index_at_col(self.cursor.row, self.cursor.col)
                    .unwrap_or_default(),
            ),
        }
    }

    pub fn handle_view_recenter(&mut self) {
        self.finish_editing();
        self.recenter_viewport = true;
        self.set_message("View recentered");
    }

    pub fn handle_line_delete(&mut self) -> orfail::Result<()> {
        self.start_editing();

        let cursor_pos = self.cursor_position();
        let current_line_cols = self.buffer.cols(cursor_pos.row);

        if cursor_pos.col >= current_line_cols {
            // Cursor is at or past end of line - delete the newline (merge with next line)
            if cursor_pos.row < self.buffer.rows().saturating_sub(1) {
                if let Some(next_line) = self.buffer.text.get(cursor_pos.row + 1).cloned() {
                    // Copy the newline to clipboard
                    self.clipboard.write("\n").or_fail()?;

                    self.buffer.text.remove(cursor_pos.row + 1);
                    if let Some(current_line) = self.buffer.text.get_mut(cursor_pos.row) {
                        current_line.extend_from_line(next_line);
                        self.buffer.dirty = true;
                    }
                    self.set_message("Killed newline");
                }
            }
        } else {
            // Delete from cursor to end of line and copy to clipboard
            if let Some(line) = self.buffer.text.get_mut(cursor_pos.row) {
                let char_index = line.char_index_at_col(cursor_pos.col);

                // Extract the text that will be deleted
                let killed_text: String = line.0[char_index..].iter().collect();

                if !killed_text.is_empty() {
                    // Copy to clipboard
                    self.clipboard.write(&killed_text).or_fail()?;

                    // Delete the text
                    line.0.truncate(char_index);
                    self.buffer.dirty = true;

                    self.set_message(format!("Killed {} characters", killed_text.len()));
                } else {
                    self.set_message("Nothing to kill");
                }
            }
        }

        Ok(())
    }

    pub fn handle_cursor_page_up(&mut self, text_area_size: tuinix::TerminalSize) {
        self.finish_editing();
        self.cursor.row = self.cursor.row.saturating_sub(text_area_size.rows);
    }

    pub fn handle_cursor_page_down(&mut self, text_area_size: tuinix::TerminalSize) {
        self.finish_editing();
        let max_row = self.buffer.rows();
        self.cursor.row = (self.cursor.row + text_area_size.rows).min(max_row);
    }
}
