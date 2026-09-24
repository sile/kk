//! The editor commands a key binding can trigger.

/// One command that the editor can carry out.
///
/// An action names *what* to do, not *how*: the core [`crate::State`] handlers
/// decide what an action means for the buffer and cursor, and the edge performs
/// any I/O an action implies (see [`BufferSave`](Action::BufferSave)).
#[derive(Debug, Clone)]
pub enum Action {
    /// Leaves the editor.
    Quit,

    /// Drops the mark, the grep mode, and the search highlight, and reports
    /// `Canceled`.
    Cancel,

    /// Persists the buffer to its file.
    BufferSave,

    /// Re-reads the buffer's file from disk, discarding unsaved edits.
    BufferReload,

    /// Undoes the most recent edit.
    BufferUndo,

    /// Moves the cursor up one row.
    CursorUp,

    /// Moves the cursor down one row.
    CursorDown,

    /// Moves the cursor left one column.
    CursorLeft,

    /// Moves the cursor right one column.
    CursorRight,

    /// Moves the cursor to the first column of its line.
    CursorLineStart,

    /// Moves the cursor to the end of its line.
    CursorLineEnd,

    /// Moves the cursor to the start of the buffer.
    CursorBufferStart,

    /// Moves the cursor to the end of the buffer.
    CursorBufferEnd,

    /// Moves the cursor up by one visible page.
    CursorPageUp,

    /// Moves the cursor down by one visible page.
    CursorPageDown,

    /// Scrolls so the cursor's line is vertically centered.
    ViewRecenter,

    /// Splits the line at the cursor.
    NewlineInsert,

    /// Inserts the key that triggered this action at the cursor.
    CharInsert,

    /// Deletes the character before the cursor.
    CharDeleteBackward,

    /// Deletes the character under the cursor.
    CharDeleteForward,

    /// Deletes the cursor's whole line.
    LineDelete,

    /// Sets the mark at the cursor.
    MarkSet,

    /// Copies the marked region to the clipboard.
    MarkCopy,

    /// Copies the marked region to the clipboard and deletes it.
    MarkCut,

    /// Inserts the clipboard's contents at the cursor.
    ClipboardPaste,

    /// Enters grep mode with the given direction.
    Grep(GrepAction),

    /// Moves the cursor to the next search hit.
    GrepNextHit,

    /// Moves the cursor to the previous search hit.
    GrepPrevHit,

    /// Reports a fixed message.
    Echo(EchoAction),

    /// Runs every enclosed action in order.
    Multiple(Vec<Action>),
}

/// The direction a [`Action::Grep`] search starts in.
#[derive(Debug, Clone)]
pub struct GrepAction {
    /// Whether the search reads forward from the cursor.
    pub forward: bool,
}

/// A message for [`Action::Echo`] to report.
#[derive(Debug, Clone)]
pub struct EchoAction {
    /// The text to show on the message line.
    pub message: String,
}
