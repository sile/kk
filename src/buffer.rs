use std::path::Path;

use orfail::OrFail;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone)]
pub enum UndoAction {
    InsertChar {
        pos: TextPosition,
        ch: char,
    },
    DeleteChar {
        pos: TextPosition,
        ch: char,
    },
    InsertNewline {
        pos: TextPosition,
    },
    DeleteNewline {
        pos: TextPosition,
        deleted_line: TextLine,
    },
    // Compound action for operations that should be undone together
    Compound(Vec<UndoAction>),
}

#[derive(Debug, Default)]
pub struct TextBuffer {
    pub text: Vec<TextLine>,
    pub dirty: bool,
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
    // Track if we're currently undoing/redoing to avoid adding to undo stack
    in_undo_redo: bool,
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
        // Clear undo/redo stacks when loading new file
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    pub fn rows(&self) -> usize {
        self.text.len()
    }

    pub fn cols(&self, row: usize) -> usize {
        self.text.get(row).map(|l| l.cols()).unwrap_or_default()
    }

    pub fn adjust_to_char_boundary(&self, mut pos: TextPosition, floor: bool) -> TextPosition {
        if let Some(line) = self.text.get(pos.row) {
            pos.col = line.adjust_to_char_boundary(pos.col, floor);
        } else {
            pos.col = 0;
        }
        pos
    }

    fn push_undo_action(&mut self, action: UndoAction) {
        if !self.in_undo_redo {
            self.undo_stack.push(action);
            // Clear redo stack when new action is performed
            self.redo_stack.clear();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> Option<TextPosition> {
        if let Some(action) = self.undo_stack.pop() {
            self.in_undo_redo = true;
            let cursor_pos = self.apply_undo_action(&action);
            self.redo_stack.push(action);
            self.in_undo_redo = false;
            self.dirty = true;
            cursor_pos
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<TextPosition> {
        if let Some(action) = self.redo_stack.pop() {
            self.in_undo_redo = true;
            let cursor_pos = self.apply_redo_action(&action);
            self.undo_stack.push(action);
            self.in_undo_redo = false;
            self.dirty = true;
            cursor_pos
        } else {
            None
        }
    }

    fn apply_undo_action(&mut self, action: &UndoAction) -> Option<TextPosition> {
        match action {
            UndoAction::InsertChar { pos, .. } => {
                // Undo insert by deleting
                self.delete_char_at_internal(*pos);
                Some(*pos)
            }
            UndoAction::DeleteChar { pos, ch } => {
                // Undo delete by inserting
                self.insert_char_at_internal(*pos, *ch);
                Some(TextPosition {
                    row: pos.row,
                    col: pos.col + ch.width().unwrap_or_default(),
                })
            }
            UndoAction::InsertNewline { pos } => {
                // Undo newline insert by joining lines
                if pos.row + 1 < self.text.len() {
                    let next_line = self.text.remove(pos.row + 1);
                    if let Some(current_line) = self.text.get_mut(pos.row) {
                        current_line.extend_from_line(next_line);
                    }
                }
                Some(*pos)
            }
            UndoAction::DeleteNewline { pos, deleted_line } => {
                // Undo newline delete by splitting line
                if let Some(current_line) = self.text.get_mut(pos.row) {
                    let chars_after = current_line.split_off_at_col(pos.col);
                    let mut new_line = deleted_line.clone();
                    new_line.0.extend(chars_after);
                    self.text.insert(pos.row + 1, new_line);
                }
                Some(TextPosition {
                    row: pos.row + 1,
                    col: 0,
                })
            }
            UndoAction::Compound(actions) => {
                // Apply compound actions in reverse order
                let mut last_pos = None;
                for action in actions.iter().rev() {
                    if let Some(pos) = self.apply_undo_action(action) {
                        last_pos = Some(pos);
                    }
                }
                last_pos
            }
        }
    }

    fn apply_redo_action(&mut self, action: &UndoAction) -> Option<TextPosition> {
        match action {
            UndoAction::InsertChar { pos, ch } => {
                // Redo insert
                self.insert_char_at_internal(*pos, *ch);
                Some(TextPosition {
                    row: pos.row,
                    col: pos.col + ch.width().unwrap_or_default(),
                })
            }
            UndoAction::DeleteChar { pos, .. } => {
                // Redo delete
                self.delete_char_at_internal(*pos);
                Some(*pos)
            }
            UndoAction::InsertNewline { pos } => {
                // Redo newline insert
                self.insert_newline_at_internal(*pos);
                Some(TextPosition {
                    row: pos.row + 1,
                    col: 0,
                })
            }
            UndoAction::DeleteNewline { pos, .. } => {
                // Redo newline delete
                if pos.row + 1 < self.text.len() {
                    let next_line = self.text.remove(pos.row + 1);
                    if let Some(current_line) = self.text.get_mut(pos.row) {
                        current_line.extend_from_line(next_line);
                    }
                }
                Some(*pos)
            }
            UndoAction::Compound(actions) => {
                // Apply compound actions in normal order
                let mut last_pos = None;
                for action in actions {
                    if let Some(pos) = self.apply_redo_action(action) {
                        last_pos = Some(pos);
                    }
                }
                last_pos
            }
        }
    }

    pub fn delete_char_at(&mut self, pos: TextPosition) -> bool {
        // Store the character for undo before deleting
        if let Some(line) = self.text.get(pos.row) {
            if let Some(ch) = line.char_at_col(pos.col) {
                self.push_undo_action(UndoAction::DeleteChar { pos, ch });
                self.delete_char_at_internal(pos);
                self.dirty = true;
                return true;
            }
        }

        // Handle forward delete at line end (merge with next line)
        if pos.col >= self.cols(pos.row) && pos.row < self.text.len().saturating_sub(1) {
            let deleted_line = self.text.get(pos.row + 1).cloned();
            if let Some(deleted_line) = deleted_line {
                self.push_undo_action(UndoAction::DeleteNewline { pos, deleted_line });
                let next_line = self.text.remove(pos.row + 1);
                if let Some(current_line) = self.text.get_mut(pos.row) {
                    current_line.extend_from_line(next_line);
                    self.dirty = true;
                    return true;
                }
            }
        }

        false
    }

    fn delete_char_at_internal(&mut self, pos: TextPosition) -> bool {
        if let Some(line) = self.text.get_mut(pos.row) {
            line.delete_char_at(pos.col)
        } else {
            false
        }
    }

    pub fn delete_char_before(&mut self, pos: TextPosition) -> Option<TextPosition> {
        if pos.col > 0 {
            // Find the character boundary before current position
            if let Some(line) = self.text.get(pos.row) {
                let char_pos = line.find_char_before(pos.col);
                if let Some(ch) = line.char_at_col(char_pos) {
                    self.push_undo_action(UndoAction::DeleteChar {
                        pos: TextPosition {
                            row: pos.row,
                            col: char_pos,
                        },
                        ch,
                    });
                    if self.delete_char_at_internal(TextPosition {
                        row: pos.row,
                        col: char_pos,
                    }) {
                        self.dirty = true;
                        return Some(TextPosition {
                            row: pos.row,
                            col: char_pos,
                        });
                    }
                }
            }
        } else if pos.row > 0 {
            // Delete newline - merge with previous line
            let current_line = self.text.get(pos.row).cloned();
            if let Some(current_line) = current_line {
                let prev_row = pos.row - 1;
                let prev_col = self.cols(prev_row);

                self.push_undo_action(UndoAction::DeleteNewline {
                    pos: TextPosition {
                        row: prev_row,
                        col: prev_col,
                    },
                    deleted_line: current_line.clone(),
                });

                self.text.remove(pos.row);
                if let Some(prev_line) = self.text.get_mut(prev_row) {
                    prev_line.extend_from_line(current_line);
                    self.dirty = true;
                    return Some(TextPosition {
                        row: prev_row,
                        col: prev_col,
                    });
                }
            }
        }
        None
    }

    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> orfail::Result<()> {
        let mut content = self
            .text
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        content.push('\n');

        std::fs::write(&path, content)
            .or_fail_with(|e| format!("failed to write file {}: {e}", path.as_ref().display()))?;

        self.dirty = false;
        Ok(())
    }

    pub fn insert_char_at(&mut self, pos: TextPosition, ch: char) -> TextPosition {
        self.push_undo_action(UndoAction::InsertChar { pos, ch });
        let new_pos = self.insert_char_at_internal(pos, ch);
        self.dirty = true;
        new_pos
    }

    fn insert_char_at_internal(&mut self, pos: TextPosition, ch: char) -> TextPosition {
        // Ensure we have enough rows
        while pos.row >= self.text.len() {
            self.text.push(TextLine::default());
        }

        if let Some(line) = self.text.get_mut(pos.row) {
            line.insert_char_at(pos.col, ch);

            // Return new cursor position
            TextPosition {
                row: pos.row,
                col: pos.col + ch.width().unwrap_or_default(),
            }
        } else {
            pos
        }
    }

    pub fn char_index_at_col(&self, row: usize, col: usize) -> Option<usize> {
        if let Some(line) = self.text.get(row) {
            let mut current_col = 0;
            for (i, &ch) in line.0.iter().enumerate() {
                if current_col >= col {
                    return Some(i);
                }
                current_col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or_default();
            }
            Some(line.0.len())
        } else {
            None
        }
    }

    pub fn insert_newline_at(&mut self, pos: TextPosition) -> TextPosition {
        self.push_undo_action(UndoAction::InsertNewline { pos });
        let new_pos = self.insert_newline_at_internal(pos);
        self.dirty = true;
        new_pos
    }

    fn insert_newline_at_internal(&mut self, pos: TextPosition) -> TextPosition {
        let current_row = pos.row;
        let current_col = pos.col;

        // Ensure we have enough rows
        while current_row >= self.text.len() {
            self.text.push(TextLine::default());
        }

        if let Some(current_line) = self.text.get_mut(current_row) {
            // Split the current line at cursor position
            let chars_after_cursor = current_line.split_off_at_col(current_col);

            // Create new line with the characters after cursor
            let new_line = TextLine::from_chars(chars_after_cursor);

            // Insert the new line after current line
            self.text.insert(current_row + 1, new_line);

            // Return new cursor position
            TextPosition {
                row: current_row + 1,
                col: 0,
            }
        } else {
            pos
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TextLine(pub Vec<char>);

impl TextLine {
    pub fn from_chars(chars: Vec<char>) -> Self {
        TextLine(chars)
    }

    pub fn to_string(&self) -> String {
        self.0.iter().collect()
    }

    pub fn extend_from_line(&mut self, other: TextLine) {
        self.0.extend(other.0);
    }

    pub fn split_off_at_col(&mut self, col: usize) -> Vec<char> {
        let char_index = self.char_index_at_col(col);
        self.0.split_off(char_index)
    }

    pub fn char_cols(&self) -> impl Iterator<Item = (usize, char)> {
        let mut col = 0;
        self.0.iter().map(move |&ch| {
            let current_col = col;
            col += ch.width().unwrap_or_default();
            (current_col, ch)
        })
    }

    // Add method to get character at column
    pub fn char_at_col(&self, col: usize) -> Option<char> {
        let mut current_col = 0;
        for &ch in &self.0 {
            if current_col == col {
                return Some(ch);
            }
            current_col += ch.width().unwrap_or_default();
            if current_col > col {
                break;
            }
        }
        None
    }

    fn cols(&self) -> usize {
        self.0.iter().filter_map(|c| c.width()).sum()
    }

    fn adjust_to_char_boundary(&self, col: usize, floor: bool) -> usize {
        let mut start = 0;
        for ch in &self.0 {
            let end = start + ch.width().unwrap_or_default();
            if start == col {
                return col;
            } else if col < end {
                return if floor { start } else { end };
            }
            start = end;
        }
        start
    }

    fn delete_char_at(&mut self, col: usize) -> bool {
        let mut current_col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if current_col == col {
                self.0.remove(i);
                return true;
            }
            current_col += ch.width().unwrap_or_default();
            if current_col > col {
                break;
            }
        }
        false
    }

    fn find_char_before(&self, col: usize) -> usize {
        let mut current_col = 0;
        for &ch in &self.0 {
            let next_col = current_col + ch.width().unwrap_or_default();
            if next_col >= col {
                return current_col;
            }
            current_col = next_col;
        }
        current_col
    }

    fn insert_char_at(&mut self, col: usize, ch: char) {
        let mut char_index = 0;
        let mut current_col = 0;

        for (i, &existing_ch) in self.0.iter().enumerate() {
            if current_col >= col {
                char_index = i;
                break;
            }
            current_col += existing_ch.width().unwrap_or_default();
            char_index = i + 1;
        }

        self.0.insert(char_index, ch);
    }

    pub fn char_index_at_col(&self, col: usize) -> usize {
        let mut current_col = 0;
        for (i, &ch) in self.0.iter().enumerate() {
            if current_col >= col {
                return i;
            }
            current_col += ch.width().unwrap_or_default();
        }
        self.0.len()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub row: usize,
    pub col: usize,
}
