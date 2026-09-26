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

    /// Drops the mark, the search mode, and the search highlight, and reports
    /// `Canceled`.
    Cancel,

    /// Persists the buffer to its file, leaves the extension context, and
    /// reports `Saved!`.
    ///
    /// The edge refuses the write when the file on disk no longer holds what
    /// the edge last read or wrote; [`BufferForceSave`](Action::BufferForceSave)
    /// is the way past that.
    BufferSave,

    /// Writes the buffer over its file whatever the file on disk holds.
    BufferForceSave,

    /// Re-reads the buffer's file from disk, discarding unsaved edits.
    BufferReload,

    /// Undoes the most recent edit.
    BufferUndo,

    /// Shows the key-binding legend if it is hidden, and hides it if it is
    /// shown.
    LegendToggle,

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

    /// Copies the marked region to the clipboard and deletes it.
    MarkCut,

    /// Inserts the clipboard's contents at the cursor.
    ClipboardPaste,

    /// Enters search mode, with `forward` giving the direction the first run
    /// reads in.
    Search {
        /// Whether the search reads forward from the cursor.
        forward: bool,
    },

    /// Moves the cursor to the next search hit.
    SearchNextHit,

    /// Moves the cursor to the previous search hit.
    SearchPrevHit,
}
