//! The editor's state and its handlers.
//!
//! Every handler is pure with respect to the outside world: it mutates [`State`]
//! and returns plain values. The two handlers that imply I/O -- saving and
//! reloading -- return the text to write and accept the text that was read, and
//! the edge performs the actual read and write (see [`crate::Action`]).

use std::collections::VecDeque;

use crate::{
    buffer::{TextBuffer, TextPosition},
    clipboard::Clipboard,
    search_mode::{Highlight, SearchMode},
};

/// The number of undo snapshots kept before the oldest is discarded.
pub const MAX_HISTORY_SIZE: usize = 1000;

/// Everything the editor knows: the buffer, the cursor, and the surrounding
/// mode state.
///
/// The fields are public so renderers and the I/O edge can read them directly;
/// edits should go through the `handle_*` methods so the undo history and the
/// viewport stay consistent.
#[derive(Debug)]
pub struct State {
    /// The cursor's position in [`buffer`](State::buffer).
    pub cursor: TextPosition,

    /// The top-left position of the visible text area.
    pub viewport: TextPosition,

    /// Whether the next viewport adjustment should center the cursor.
    pub recenter_viewport: bool,

    /// The text being edited.
    pub buffer: TextBuffer,

    /// A message to show once, cleared after the next render.
    pub message: Option<String>,

    /// The mark, when a region has been started.
    pub mark: Option<TextPosition>,

    /// The clipboard text cut from or copied out of the buffer.
    pub clipboard: Clipboard,

    /// Whether an edit has already been recorded in [`history`](State::history).
    pub editing: bool,

    /// Whether the last command was a kill, which is what makes a run of kills
    /// collect into one clipboard entry rather than replacing it.
    ///
    /// [`handle_line_delete`](State::handle_line_delete) sets it, and both
    /// `start_editing` and `finish_editing` clear it, so any edit or cursor
    /// move breaks the run.
    kill_chained: bool,

    /// Undo snapshots, oldest first.
    pub history: VecDeque<(TextPosition, TextBuffer)>,

    /// How many entries of [`history`](State::history) are still reachable by
    /// undo.
    pub undo_index: usize,

    /// The active search prompt, if one is open. // TODO: non-optional
    pub search_mode: Option<SearchMode>,

    /// Where the cursor and viewport were when the search prompt opened.
    ///
    /// A search moves the cursor to a hit and the viewport to follow it;
    /// leaving the prompt with `C-g` puts both back, so an abandoned search
    /// leaves no trace. The prompt is always opened through
    /// [`handle_search_enter`](State::handle_search_enter), which fills this in.
    search_return: Option<(TextPosition, TextPosition)>,

    /// The current search matches, used for highlighting.
    pub highlight: Highlight,
}

impl State {
    /// Builds the editor state around an already-loaded `buffer`.
    pub fn new(buffer: TextBuffer) -> Self {
        Self {
            cursor: TextPosition::default(),
            viewport: TextPosition::default(),
            recenter_viewport: false,
            buffer,
            message: None,
            mark: None,
            clipboard: Clipboard::default(),
            editing: false,
            kill_chained: false,
            history: VecDeque::new(),
            undo_index: 0,
            search_mode: None,
            search_return: None,
            highlight: Highlight::default(),
        }
    }

    /// Queues `message` to be shown once, on the next render.
    pub fn set_message<S: Into<String>>(&mut self, message: S) {
        self.message = Some(message.into());
    }

    /// Returns the cursor's position relative to the visible text area, which
    /// is where the terminal cursor belongs.
    pub fn terminal_cursor_position(&self) -> tuinix::Position {
        let pos = self.cursor_position();
        let screen_row = pos.row.saturating_sub(self.viewport.row);
        let screen_col = pos.col.saturating_sub(self.viewport.col);
        tuinix::Position {
            row: screen_row,
            col: screen_col,
        }
    }

    /// Returns the cursor's position, snapped back onto a character boundary.
    pub fn cursor_position(&self) -> TextPosition {
        self.buffer.adjust_to_char_boundary(self.cursor, true)
    }

    /// Scrolls the viewport just far enough to keep the cursor visible.
    ///
    /// When [`recenter_viewport`](State::recenter_viewport) is set, the cursor
    /// is centered instead and the flag is cleared.
    pub fn adjust_viewport(&mut self, text_area_size: tuinix::Size) {
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
        // Any edit that starts here breaks a run of kills, including the ones
        // that continue an undo run and so never call `finish_editing`.
        // `handle_line_delete` reads the flag before this and re-arms it after,
        // which is what lets one kill chain onto the next.
        self.kill_chained = false;

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

    /// Ends the current edit run, so the next edit records a new snapshot.
    ///
    /// Also ends a run of kills. Together with `start_editing`, which clears the
    /// same flag, this covers every break: the edits that only start an edit
    /// (inserting, deleting a character) and the moves and commands that only
    /// end one (moving the cursor, undoing).
    /// [`handle_line_delete`](State::handle_line_delete) reads the flag before
    /// its own `start_editing` and re-arms it after its `finish_editing`, which
    /// is what chains one kill onto the next.
    pub fn finish_editing(&mut self) {
        self.editing = false;
        self.kill_chained = false;
    }

    /// Moves the cursor up one row.
    pub fn handle_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(1);
        self.finish_editing();
    }

    /// Moves the cursor to `row`/`col` of the buffer.
    ///
    /// Both are absolute and 0-based. Out-of-range positions are clamped rather
    /// than refused: the row is clamped to the buffer and the column to the
    /// target line, and the column is then snapped back onto a character
    /// boundary. The viewport follows on the next
    /// [`adjust_viewport`](Self::adjust_viewport).
    pub fn handle_cursor_to_position(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.buffer.rows());
        self.cursor.col = self.buffer.cols(self.cursor.row).min(col);
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        self.finish_editing();
    }

    /// Moves the cursor to the character a click at `row`/`col` of the visible
    /// text area landed on.
    ///
    /// Both are relative to the text area, not the terminal, so the viewport is
    /// added here rather than by the caller.
    pub fn handle_cursor_to_screen_position(&mut self, row: usize, col: usize) {
        self.cursor.row = (self.viewport.row + row).min(self.buffer.rows());
        self.cursor.col = self
            .buffer
            .cols(self.cursor.row)
            .min(self.viewport.col + col);
        self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
        self.finish_editing();
    }

    /// Scrolls the viewport and the cursor `rows` lines down, or up when
    /// `rows` is negative.
    ///
    /// The cursor moves with the viewport because [`adjust_viewport`](Self::adjust_viewport)
    /// pulls the viewport back to the cursor on the next render, so a viewport
    /// moved on its own would snap right back.
    pub fn handle_scroll(&mut self, rows: isize) {
        if rows < 0 {
            for _ in rows..0 {
                self.handle_cursor_up();
            }
        } else {
            for _ in 0..rows {
                self.handle_cursor_down();
            }
        }
    }

    /// Moves the cursor down one row.
    pub fn handle_cursor_down(&mut self) {
        self.cursor.row = self.cursor.row.saturating_add(1).min(self.buffer.rows());
        self.finish_editing();
    }

    /// Moves the cursor left one column, wrapping to the previous line's end.
    ///
    /// While a search prompt is open, this moves the query's insertion cursor
    /// instead.
    pub fn handle_cursor_left(&mut self) {
        if let Some(search) = &mut self.search_mode {
            search.cursor = search.cursor.saturating_sub(1);
            return;
        }

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

    /// Moves the cursor right one column, wrapping to the next line's start.
    ///
    /// While a search prompt is open, this moves the query's insertion cursor
    /// instead.
    pub fn handle_cursor_right(&mut self) {
        if let Some(search) = &mut self.search_mode {
            search.cursor = (search.cursor + 1).min(search.query.len());
            return;
        }

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

    /// Moves the cursor to its line's first column.
    pub fn handle_cursor_line_start(&mut self) {
        if let Some(search) = &mut self.search_mode {
            search.cursor = 0;
            return;
        }

        self.cursor.col = 0;
        self.finish_editing();
    }

    /// Moves the cursor to its line's end.
    pub fn handle_cursor_line_end(&mut self) {
        if let Some(search) = &mut self.search_mode {
            search.cursor = search.query.len();
            return;
        }

        self.cursor.col = self.buffer.cols(self.cursor.row);
        self.finish_editing();
    }

    /// Moves the cursor to the start of the buffer.
    pub fn handle_cursor_buffer_start(&mut self) {
        self.cursor = TextPosition::default();
        self.finish_editing();
    }

    /// Moves the cursor to the last line of the buffer.
    pub fn handle_cursor_buffer_end(&mut self) {
        self.cursor.row = self.buffer.rows();
        self.cursor.col = 0;
        self.finish_editing();
    }

    /// Deletes the character before the cursor.
    ///
    /// While a search prompt is open, this deletes from the query instead and
    /// re-runs it.
    pub fn handle_char_delete_backward(&mut self) {
        if let Some(search) = &mut self.search_mode {
            if search.cursor > 0 {
                search.query.remove(search.cursor - 1);
                search.cursor -= 1;
                self.rerun_query();
            }
            return;
        }

        self.start_editing();
        if let Some(new_pos) = self.buffer.delete_char_before(self.cursor) {
            self.cursor = new_pos;
        }
    }

    /// Deletes the character under the cursor.
    ///
    /// While a search prompt is open, this deletes from the query instead and
    /// re-runs it.
    pub fn handle_char_delete_forward(&mut self) {
        if let Some(search) = &mut self.search_mode {
            if search.cursor < search.query.len() {
                search.query.remove(search.cursor);
                self.rerun_query();
            }
            return;
        }

        self.start_editing();
        self.buffer.delete_char_at(self.cursor);
    }

    /// Recomputes the highlight for the query typed so far.
    ///
    /// It only refreshes the matches: the cursor stays where it is until `C-s`
    /// or `C-r` asks for a hit, so typing a query does not drag the buffer
    /// around under the prompt.
    fn rerun_query(&mut self) {
        let Some(search) = &self.search_mode else {
            return;
        };
        self.highlight = search.search(&self.buffer);
    }

    /// Renders the buffer for saving and returns the text to persist.
    ///
    /// The caller (the I/O edge) writes it and then calls [`Self::report_saved`]
    /// once the write succeeded; the core never touches the file system.
    pub fn handle_buffer_save(&mut self) -> String {
        self.set_message("Saving");
        self.buffer.to_text()
    }

    /// Reports that the edge wrote `chars` characters out.
    pub fn report_saved(&mut self, chars: usize) {
        self.set_message(format!("Saved {chars} chars"));
    }

    /// Replaces the buffer with `text`, as the edge would after reading the file
    /// back, and adjusts the cursor so it stays on a valid position.
    pub fn handle_buffer_reload(&mut self, text: &str) {
        self.finish_editing();
        self.start_editing();

        self.buffer.replace_from_text(text);

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

        self.set_message("Reloaded");
        self.finish_editing();
    }

    /// Inserts `ch` at the cursor.
    ///
    /// While a search prompt is open, the character enters the query instead
    /// and re-runs it.
    pub fn handle_char_insert(&mut self, ch: char) {
        if let Some(search) = &mut self.search_mode {
            search.insert_char(ch);
            self.rerun_query();
            return;
        }

        self.start_editing();
        self.cursor = self.buffer.insert_char_at(self.cursor, ch);
    }

    /// Splits the line at the cursor.
    pub fn handle_newline_insert(&mut self) {
        self.finish_editing();
        self.start_editing();
        self.cursor = self.buffer.insert_newline_at(self.cursor);
        self.finish_editing();
    }

    /// Restores the buffer and cursor from the previous history snapshot.
    ///
    /// An edit still in progress is closed first: the snapshot it already
    /// pushed is the state to come back to, so nothing new is recorded.
    /// Reports `Nothing to undo` when there is nothing left to restore.
    pub fn handle_buffer_undo(&mut self) {
        self.finish_editing();

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

    /// Sets the mark at the cursor, or clears it when already there.
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

    /// Copies the region between the mark and the cursor to the clipboard and
    /// deletes it, leaving the cursor at the region's start.
    ///
    /// The mark is cleared either way. Reports `No mark set` when there is no
    /// mark, and `Nothing to cut` when the region is empty.
    pub fn handle_mark_cut(&mut self) {
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

                self.clipboard.write(&text);
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
                    if start_char_idx < line.0.len() {
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

            if start.row + 1 < self.buffer.text.len()
                && let Some(end_line) = self.buffer.text.get(start.row + 1).cloned()
            {
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

    /// Inserts the clipboard's contents at the cursor.
    ///
    /// Newlines in the contents become line breaks. While a search prompt is
    /// open, the contents enter the query instead and re-run it.
    pub fn handle_clipboard_paste(&mut self) {
        if let Some(search) = &mut self.search_mode {
            let text = self.clipboard.read();

            if text.is_empty() {
                self.set_message("Clipboard is empty");
                return;
            }

            // Insert clipboard text at current cursor position in search query
            for ch in text.chars() {
                // Skip control characters and newlines in search query
                if !ch.is_control() {
                    search.query.insert(search.cursor, ch);
                    search.cursor += 1;
                }
            }

            // Re-run the search with updated query
            self.rerun_query();
            return;
        };

        self.finish_editing();

        let text = self.clipboard.read();

        if text.is_empty() {
            self.set_message("Clipboard is empty");
            return;
        }

        // Split text into lines
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            self.set_message("Nothing to paste");
            return;
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
    }

    /// Asks the next viewport adjustment to center the cursor.
    pub fn handle_view_recenter(&mut self) {
        self.finish_editing();
        self.recenter_viewport = true;
        self.set_message("View recentered");
    }

    /// Deletes from the cursor to the end of the line, or joins the next line
    /// when the cursor is already at the end.
    ///
    /// The removed text goes to the clipboard. A kill that follows another one
    /// without a break in between is chained onto the entry the previous kill
    /// left rather than replacing it, so a run of `C-k` collects what it
    /// removed into one entry (see [`finish_editing`](State::finish_editing) for
    /// what counts as a break).
    pub fn handle_line_delete(&mut self) {
        let append = self.kill_chained;
        self.start_editing();

        let cursor_pos = self.cursor_position();
        let current_line_cols = self.buffer.cols(cursor_pos.row);

        if cursor_pos.col >= current_line_cols {
            // Cursor is at or past end of line - delete the newline (merge with next line)
            if cursor_pos.row < self.buffer.rows().saturating_sub(1)
                && let Some(next_line) = self.buffer.text.get(cursor_pos.row + 1).cloned()
            {
                // Copy the newline to clipboard
                if append {
                    self.clipboard.append("\n");
                } else {
                    self.clipboard.write("\n");
                }

                self.buffer.text.remove(cursor_pos.row + 1);
                if let Some(current_line) = self.buffer.text.get_mut(cursor_pos.row) {
                    current_line.extend_from_line(next_line);
                }
                if append {
                    self.set_message("Appended newline");
                } else {
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
                    if append {
                        self.clipboard.append(&killed_text);
                    } else {
                        self.clipboard.write(&killed_text);
                    }

                    // Delete the text
                    line.0.truncate(char_index);

                    if append {
                        self.set_message(format!("Appended {} characters", killed_text.len()));
                    } else {
                        self.set_message(format!("Killed {} characters", killed_text.len()));
                    }
                } else {
                    self.set_message("Nothing to kill");
                }
            }
        }

        // Re-arm the flag so the next kill chains onto this one. This runs for a
        // no-op kill too: what chains is the command, not its effect.
        self.finish_editing();
        self.kill_chained = true;
    }

    /// Drops the mark and the search state, and reports `Canceled`.
    ///
    /// This is the `C-g` of the contexts that hold no prompt; a prompt is left
    /// through [`handle_search_cancel`](State::handle_search_cancel).
    pub fn handle_cancel(&mut self) {
        self.clear_transient_state();
        self.set_message("Canceled");
    }

    /// Drops the mark and every trace of a search, prompt included.
    ///
    /// Commands that leave the editing mode behind -- cancelling, saving --
    /// call this, so a search cannot outlive the context that opened it. What
    /// the cursor does is the caller's business: cancelling puts it back and
    /// saving leaves it where it is.
    pub fn clear_transient_state(&mut self) {
        self.search_return = None;
        self.finish_editing();
        self.mark = None;
        self.search_mode = None;
        self.highlight = Highlight::default();
    }

    /// Opens the search prompt with an empty query.
    ///
    /// The cursor and viewport are remembered first, so
    /// [`handle_search_cancel`](State::handle_search_cancel) can put the buffer
    /// back where the search found it. The mark and the previous highlight are
    /// dropped: they belong to the editing that the prompt interrupts.
    pub fn handle_search_enter(&mut self) {
        self.clear_transient_state();
        self.search_return = Some((self.cursor, self.viewport));
        self.search_mode = Some(SearchMode::new());
        self.set_message("Entered search mode");
    }

    /// Abandons the search prompt and returns to where it was opened.
    ///
    /// The cursor and viewport go back to their remembered positions, so a
    /// search that was merely looked at leaves the buffer untouched. Does
    /// nothing when no prompt is open.
    pub fn handle_search_cancel(&mut self) {
        if self.search_mode.is_none() {
            return;
        }

        if let Some((cursor, viewport)) = self.search_return.take() {
            self.cursor = cursor;
            self.viewport = viewport;
        }

        self.clear_transient_state();
        self.set_message("Canceled");
    }

    /// Accepts the search prompt and stays on the hit the cursor sits on.
    ///
    /// Only the prompt and the highlight go away: the cursor keeps the position
    /// the hit commands gave it. Does nothing when no prompt is open.
    pub fn handle_search_accept(&mut self) {
        if self.search_mode.is_none() {
            return;
        }

        self.clear_transient_state();
    }

    /// Moves the cursor to the next match after it, wrapping to the first.
    ///
    /// Does nothing when no search prompt is open.
    pub fn handle_search_next_hit(&mut self) {
        if self.search_mode.is_none() {
            return;
        }

        self.finish_editing();

        let current_pos = self.cursor_position();

        // Find the next highlight item after the current cursor position
        if let Some(next_item) = self
            .highlight
            .items
            .iter()
            .find(|item| item.start_position > current_pos)
        {
            self.cursor = next_item.start_position;
            self.recenter_viewport = true;
            self.set_message("Moved to next search hit");
        } else if let Some(first_item) = self.highlight.items.first() {
            // Wrap around to the first item
            self.cursor = first_item.start_position;
            self.recenter_viewport = true;
            self.set_message("Wrapped to first search hit");
        }
    }

    /// Moves the cursor to the previous match before it, wrapping to the last.
    ///
    /// Does nothing when no search prompt is open.
    pub fn handle_search_prev_hit(&mut self) {
        if self.search_mode.is_none() {
            return;
        }

        self.finish_editing();

        let current_pos = self.cursor_position();

        // Find the previous highlight item before the current cursor position
        if let Some(prev_item) = self
            .highlight
            .items
            .iter()
            .rev()
            .find(|item| item.start_position < current_pos)
        {
            self.cursor = prev_item.start_position;
            self.recenter_viewport = true;
            self.set_message("Moved to previous search hit");
        } else if let Some(last_item) = self.highlight.items.last() {
            // Wrap around to the last item
            self.cursor = last_item.start_position;
            self.recenter_viewport = true;
            self.set_message("Wrapped to last search hit");
        }
    }
}
