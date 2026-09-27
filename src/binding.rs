//! Input modes and the `match`-based resolvers that map input to actions.

use crate::action::Action;

/// The legend rows for the edit mode, in legend order, the bottom border last.
///
/// Each row is a whole row of the legend, written out as the user reads it: the
/// border stroke, the chord, the spaces that separate it from its label, and the
/// label. Only the left border is drawn, so no row is closed on the right, and
/// the last row is the bottom border with the mode title centered in it. The
/// bindings are hard-coded, so this table is too, and the two are kept in step
/// by hand.
pub const EDIT_LEGEND: &[&str] = &[
    "\u{2502}C-c quit",
    "\u{2502}C-g cancel",
    "\u{2502}C-x ext",
    "\u{2502}C-s search",
    "\u{2502}C-  mark",
    "\u{2502}C-w cut",
    "\u{2502}C-k cut-line",
    "\u{2502}C-y paste",
    "\u{2502}C-u undo",
    "\u{2502}C-a bol",
    "\u{2502}C-e eol",
    "\u{2502}C-p \u{2191} up",
    "\u{2502}C-n \u{2193} down",
    "\u{2502}C-b \u{2190} left",
    "\u{2502}C-f \u{2192} right",
    "\u{2502}C-h \u{232b} bs",
    "\u{2502}C-d \u{2326} delete",
    "\u{2502}C-l recenter",
    "\u{2514}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}",
];

/// The legend rows for the [search mode](crate::Mode::Search), in legend order,
/// the bottom border last.
pub const SEARCH_LEGEND: &[&str] = &[
    "\u{2502}C-g cancel",
    "\u{2502}C-r \u{21e4} prev",
    "\u{2502}C-s \u{21e5} next",
    "\u{2502}C-k cut-line",
    "\u{2502}C-y paste",
    "\u{2502}C-a bol",
    "\u{2502}C-e eol",
    "\u{2502}C-b \u{2190} left",
    "\u{2502}C-f \u{2192} right",
    "\u{2502}C-h \u{232b} bs",
    "\u{2502}C-d \u{2326} delete",
    "\u{2514}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}",
];

/// The legend rows for the extension mode, in legend order, the bottom border
/// last.
pub const EXT_LEGEND: &[&str] = &[
    "\u{2502}C-g cancel",
    "\u{2502}s   save",
    "\u{2502}S   force-save",
    "\u{2502}r   reload",
    "\u{2502}a   bof",
    "\u{2502}e   eof",
    "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
];

/// Identifies one of the built-in input modes.
///
/// A mode selects which binding table an input is resolved against, and a
/// resolved input may carry the mode to move to. [`Search`](Mode::Search) and
/// [`Ext`](Mode::Ext) are entered by a chord and left by another: saving,
/// reloading, or cancelling leaves [`Ext`](Mode::Ext).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// The default editing mode.
    Edit,

    /// The mode active while a search prompt is collecting a query; its
    /// bindings edit the query and move between hits.
    Search,

    /// The mode `C-x` enters, holding the buffer-level chords.
    Ext,
}

impl Mode {
    /// Returns the legend rows of the mode, in legend order.
    pub fn legend(self) -> &'static [&'static str] {
        match self {
            Mode::Edit => EDIT_LEGEND,
            Mode::Search => SEARCH_LEGEND,
            Mode::Ext => EXT_LEGEND,
        }
    }

    /// Returns the size the legend of the mode needs, limited to `limit`.
    ///
    /// The width is the widest legend row and the height is the number of rows,
    /// the bottom border included. Every row of the box is this wide, so a caller
    /// can size a frame to hold it whole. A limit smaller than the legend reports
    /// the limit, so a caller that compares the result against the limit can tell
    /// the legend was clipped.
    ///
    /// # Examples
    ///
    /// ```
    /// let room = tuinix::Size { rows: 40, cols: 100 };
    /// let ext = kk::Mode::Ext.legend_size(room);
    /// assert_eq!(ext.rows, 7);
    /// assert_eq!(ext.cols, 15);
    /// ```
    pub fn legend_size(self, limit: tuinix::Size) -> tuinix::Size {
        let rows = self.legend().len();
        let cols = self
            .legend()
            .iter()
            .map(|row| crate::terminal::str_cols(row))
            .max()
            .unwrap_or(0);
        tuinix::Size {
            rows: rows.min(limit.rows),
            cols: cols.min(limit.cols),
        }
    }

    /// Resolves `input` in the mode to the action and mode switch it means.
    ///
    /// Returns `None` when nothing in the mode is bound to that input; callers
    /// report that to the user.
    ///
    /// The input has to be a key: no mouse, paste, or unrecognized input is bound.
    pub fn resolve(self, input: &tuinix::Input) -> Option<Resolved> {
        let key = match input {
            tuinix::Input::Key(key) => key,
            _ => return None,
        };

        match self {
            Mode::Edit => resolve_edit(key),
            Mode::Search => resolve_search(key),
            Mode::Ext => resolve_ext(key),
        }
    }
}

/// What a single terminal input does: an action to run and a mode to switch to.
/// Either field may be `None`.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The action to carry out, if any.
    pub action: Option<Action>,

    /// The mode to switch to, if any.
    pub mode: Option<Mode>,
}

/// Runs `action` and stays in the current mode.
fn act(action: Action) -> Resolved {
    Resolved {
        action: Some(action),
        mode: None,
    }
}

/// Runs `action` and then switches to `mode`.
fn then(action: Action, mode: Mode) -> Resolved {
    Resolved {
        action: Some(action),
        mode: Some(mode),
    }
}

/// Ends the prompt and restores the edit mode.
fn cancel() -> Resolved {
    then(Action::Cancel, Mode::Edit)
}

fn resolve_edit(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, code, .. } = *key;

    Some(match (ctrl, code) {
        (true, tuinix::KeyCode::Char('c')) => act(Action::Quit),
        (true, tuinix::KeyCode::Char('g')) => cancel(),
        (true, tuinix::KeyCode::Char('s')) => then(Action::SearchEnter, Mode::Search),
        (true, tuinix::KeyCode::Char('x')) => then(Action::ExtEnter, Mode::Ext),
        (true, tuinix::KeyCode::Char('y')) => act(Action::ClipboardPaste),
        (true, tuinix::KeyCode::Char('w')) => act(Action::MarkCut),
        (true, tuinix::KeyCode::Char('u')) => act(Action::BufferUndo),
        // A lone `ESC` is held back by the decoder and then committed as
        // `Escape` with no modifiers, so the plain code is the whole chord.
        (false, tuinix::KeyCode::Escape) => act(Action::LegendToggle),
        (true, tuinix::KeyCode::Char(' ' | '`')) => act(Action::MarkSet),
        (true, tuinix::KeyCode::Char('l')) => act(Action::ViewRecenter),
        (true, tuinix::KeyCode::Char('k')) => act(Action::LineDelete),
        (true, tuinix::KeyCode::Char('a')) => act(Action::CursorLineStart),
        (true, tuinix::KeyCode::Char('e')) => act(Action::CursorLineEnd),
        (true, tuinix::KeyCode::Char('d')) => act(Action::CharDeleteForward),
        (true, tuinix::KeyCode::Char('h')) => act(Action::CharDeleteBackward),
        (true, tuinix::KeyCode::Char('p')) => act(Action::CursorUp),
        (true, tuinix::KeyCode::Char('n')) => act(Action::CursorDown),
        (true, tuinix::KeyCode::Char('b')) => act(Action::CursorLeft),
        (true, tuinix::KeyCode::Char('f')) => act(Action::CursorRight),
        (false, tuinix::KeyCode::Delete) => act(Action::CharDeleteForward),
        (false, tuinix::KeyCode::Backspace) => act(Action::CharDeleteBackward),
        (false, tuinix::KeyCode::Enter) => act(Action::NewlineInsert),
        (false, tuinix::KeyCode::Up) => act(Action::CursorUp),
        (false, tuinix::KeyCode::Down) => act(Action::CursorDown),
        (false, tuinix::KeyCode::Left) => act(Action::CursorLeft),
        (false, tuinix::KeyCode::Right) => act(Action::CursorRight),
        (true, tuinix::KeyCode::Left) => act(Action::CursorUp),
        (true, tuinix::KeyCode::Right) => act(Action::CursorDown),
        // Any other bare, non-control character is text, not a binding.
        (false, tuinix::KeyCode::Char(ch)) if !ch.is_control() => act(Action::CharInsert),
        _ => return None,
    })
}

fn resolve_search(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, code, .. } = *key;

    Some(match (ctrl, code) {
        (true, tuinix::KeyCode::Char('g')) => then(Action::SearchCancel, Mode::Edit),
        (false, tuinix::KeyCode::Enter) => then(Action::SearchAccept, Mode::Edit),
        (true, tuinix::KeyCode::Char('y')) => act(Action::ClipboardPaste),
        (true, tuinix::KeyCode::Char('k')) => act(Action::SearchKillQuery),
        (true, tuinix::KeyCode::Char('s')) => act(Action::SearchNextHit),
        (true, tuinix::KeyCode::Char('r')) => act(Action::SearchPrevHit),
        (false, tuinix::KeyCode::Tab) => act(Action::SearchNextHit),
        (false, tuinix::KeyCode::BackTab) => act(Action::SearchPrevHit),
        (true, tuinix::KeyCode::Char('a')) => act(Action::CursorLineStart),
        (true, tuinix::KeyCode::Char('e')) => act(Action::CursorLineEnd),
        (true, tuinix::KeyCode::Char('d')) => act(Action::CharDeleteForward),
        (true, tuinix::KeyCode::Char('h')) => act(Action::CharDeleteBackward),
        (true, tuinix::KeyCode::Char('b')) => act(Action::CursorLeft),
        (true, tuinix::KeyCode::Char('f')) => act(Action::CursorRight),
        (false, tuinix::KeyCode::Escape) => act(Action::LegendToggle),
        (false, tuinix::KeyCode::Delete) => act(Action::CharDeleteForward),
        (false, tuinix::KeyCode::Backspace) => act(Action::CharDeleteBackward),
        (false, tuinix::KeyCode::Left) => act(Action::CursorLeft),
        (false, tuinix::KeyCode::Right) => act(Action::CursorRight),
        // Any other bare, non-control character is part of the query.
        (false, tuinix::KeyCode::Char(ch)) if !ch.is_control() => act(Action::CharInsert),
        _ => return None,
    })
}

fn resolve_ext(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, code, .. } = *key;

    // The extension mode is entered with `C-x`, so its own chords drop the
    // ctrl prefix: a bare `s` saves. Only `C-g` keeps its ctrl, matching every
    // other mode, and case is what separates save from force-save.
    Some(match (ctrl, code) {
        (true, tuinix::KeyCode::Char('g')) => cancel(),
        (false, tuinix::KeyCode::Char('S')) => then(Action::BufferForceSave, Mode::Edit),
        (false, tuinix::KeyCode::Char('s')) => then(Action::BufferSave, Mode::Edit),
        (false, tuinix::KeyCode::Char('r')) => then(Action::BufferReload, Mode::Edit),
        (false, tuinix::KeyCode::Char('a')) => then(Action::CursorBufferStart, Mode::Edit),
        (false, tuinix::KeyCode::Char('e')) => then(Action::CursorBufferEnd, Mode::Edit),
        (false, tuinix::KeyCode::Escape) => act(Action::LegendToggle),
        _ => return None,
    })
}
