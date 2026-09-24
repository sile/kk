//! Input contexts and the `match`-based resolvers that map input to actions.

use crate::action::{Action, GrepAction};

/// The vertical box-drawing stroke between a legend row and its label.
pub const BORDER_VERTICAL: &str = "\u{2502}";

/// The horizontal box-drawing stroke of the legend's bottom border.
pub const BORDER_HORIZONTAL: &str = "\u{2500}";

/// The title the legend shows for `context`.
///
/// This is the context spelled out, so the box is labelled `main` or `grep`
/// rather than by an enum name. It is centered in the bottom border, and a title
/// too wide for the box is dropped.
pub fn title(context: Context) -> &'static str {
    match context {
        Context::Main => "main",
        Context::Grep => "grep",
        Context::Ext => "ext",
    }
}

/// The legend rows for the main context, in legend order.
///
/// Each row is a whole row of the legend, written out as the user reads it: the
/// border stroke, the chord, the spaces that separate it from its label, and the
/// label. Only the left border is drawn. The bindings are hard-coded, so this
/// table is too, and the two are kept in step by hand.
pub const MAIN_LEGEND: &[&str] = &[
    "\u{2502} C-c quit",
    "\u{2502} C-g cancel",
    "\u{2502} C-r rgrep",
    "\u{2502} C-s grep",
    "\u{2502} C-x ext",
    "\u{2502} C-y paste",
    "\u{2502} C-w cut",
    "\u{2502} C-  mark",
    "\u{2502} M-r reload",
    "\u{2502} C-l recenter",
    "\u{2502} C-k kill-line",
    "\u{2502} C-a line-start",
    "\u{2502} C-e line-end",
    "\u{2502} M-< buffer-start",
    "\u{2502} M-> buffer-end",
    "\u{2502} C-p up",
    "\u{2502} C-n down",
    "\u{2502} C-b left",
    "\u{2502} C-f right",
    "\u{2502} C-j newline",
    "\u{2502} C-h backspace",
    "\u{2502} C-d delete",
    "\u{2502} C-/ undo",
];

/// The legend rows for the grep context, in legend order.
pub const GREP_LEGEND: &[&str] = &[
    "\u{2502} C-g cancel",
    "\u{2502} C-s next-hit",
    "\u{2502} C-r prev-hit",
    "\u{2502} C-y paste",
    "\u{2502} C-a line-start",
    "\u{2502} C-e line-end",
    "\u{2502} C-b left",
    "\u{2502} C-f right",
    "\u{2502} C-h backspace",
    "\u{2502} C-d delete",
];

/// The legend rows for the extension context, in legend order.
pub const EXT_LEGEND: &[&str] = &["\u{2502} C-g cancel", "\u{2502} C-s save"];

/// Returns the legend rows of `context`, in legend order.
pub fn legend(context: Context) -> &'static [&'static str] {
    match context {
        Context::Main => MAIN_LEGEND,
        Context::Grep => GREP_LEGEND,
        Context::Ext => EXT_LEGEND,
    }
}

/// The columns a rendered legend occupies.
///
/// The width is the widest legend row, and the height is one row per binding
/// plus one for the bottom border. Every row of the box is this wide, so a
/// caller can size a frame to hold it whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegendSize {
    /// The box width in columns.
    pub cols: usize,

    /// The box height in rows, the bottom border included.
    pub rows: usize,
}

/// Returns the size the legend of `context` needs, limited to `limit`.
///
/// A limit smaller than the legend reports the limit, so a caller that compares
/// the result against the limit can tell the legend was clipped.
///
/// # Examples
///
/// ```
/// let room = tuinix::Size { rows: 40, cols: 100 };
/// let ext = kk::legend_size(kk::Context::Ext, room);
/// assert_eq!(ext.rows, 3);
/// assert_eq!(ext.cols, 12);
/// ```
pub fn legend_size(context: Context, limit: tuinix::Size) -> LegendSize {
    let rows = legend(context).len() + 1;
    let cols = legend(context)
        .iter()
        .map(|row| crate::terminal::str_cols(row))
        .max()
        .unwrap_or(0);
    LegendSize {
        cols: cols.min(limit.cols),
        rows: rows.min(limit.rows),
    }
}

/// Identifies one of the built-in input contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Context {
    /// The default editing context.
    Main,

    /// The context active while grep mode is collecting a query.
    Grep,

    /// A context reserved for extensions.
    Ext,
}

/// What a single terminal input does: an action to run and a context to switch
/// to. Either field may be `None`.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// The action to carry out, if any.
    pub action: Option<Action>,

    /// The context to switch to, if any.
    pub context: Option<Context>,
}

/// Resolves `input` in `context` to the action and context switch it means.
///
/// Returns `None` when nothing in the context is bound to that input; callers
/// report that to the user.
///
/// The input has to be a key: no mouse, paste, or unrecognized input is bound.
pub fn resolve(context: Context, input: &tuinix::Input) -> Option<Resolved> {
    let key = match input {
        tuinix::Input::Key(key) => key,
        _ => return None,
    };

    match context {
        Context::Main => resolve_main(key),
        Context::Grep => resolve_grep(key),
        Context::Ext => resolve_ext(key),
    }
}

/// Runs `action` and stays in the current context.
fn act(action: Action) -> Resolved {
    Resolved {
        action: Some(action),
        context: None,
    }
}

/// Runs `action` and then switches to `context`.
fn then(action: Action, context: Context) -> Resolved {
    Resolved {
        action: Some(action),
        context: Some(context),
    }
}

/// Switches to `context` without running anything.
fn only(context: Context) -> Resolved {
    Resolved {
        action: None,
        context: Some(context),
    }
}

/// Ends the prompt and restores the main context.
fn cancel() -> Resolved {
    then(Action::Cancel, Context::Main)
}

fn resolve_main(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, alt, code } = *key;

    Some(match (ctrl, alt, code) {
        (true, false, tuinix::KeyCode::Char('c')) => act(Action::Quit),
        (true, false, tuinix::KeyCode::Char('g')) => cancel(),
        (true, false, tuinix::KeyCode::Char('r')) => {
            then(Action::Grep(GrepAction { forward: false }), Context::Grep)
        }
        (true, false, tuinix::KeyCode::Char('s')) => {
            then(Action::Grep(GrepAction { forward: true }), Context::Grep)
        }
        (true, false, tuinix::KeyCode::Char('x')) => only(Context::Ext),
        (true, false, tuinix::KeyCode::Char('y')) => act(Action::ClipboardPaste),
        (true, false, tuinix::KeyCode::Char('w')) => act(Action::MarkCut),
        (false, true, tuinix::KeyCode::Char('r')) => act(Action::BufferReload),
        (false, true, tuinix::KeyCode::Char('<')) => act(Action::CursorBufferStart),
        (false, true, tuinix::KeyCode::Char('>')) => act(Action::CursorBufferEnd),
        (true, false, tuinix::KeyCode::Char('/' | 'u' | '\u{7f}')) => act(Action::BufferUndo),
        (true, false, tuinix::KeyCode::Char(' ' | '`')) => act(Action::MarkSet),
        (true, false, tuinix::KeyCode::Char('l')) => act(Action::ViewRecenter),
        (true, false, tuinix::KeyCode::Char('k')) => act(Action::LineDelete),
        (true, false, tuinix::KeyCode::Char('a')) => act(Action::CursorLineStart),
        (true, false, tuinix::KeyCode::Char('e')) => act(Action::CursorLineEnd),
        (true, false, tuinix::KeyCode::Char('d')) => act(Action::CharDeleteForward),
        (true, false, tuinix::KeyCode::Char('h')) => act(Action::CharDeleteBackward),
        (true, false, tuinix::KeyCode::Char('j')) => act(Action::NewlineInsert),
        (true, false, tuinix::KeyCode::Char('p')) => act(Action::CursorUp),
        (true, false, tuinix::KeyCode::Char('n')) => act(Action::CursorDown),
        (true, false, tuinix::KeyCode::Char('b')) => act(Action::CursorLeft),
        (true, false, tuinix::KeyCode::Char('f')) => act(Action::CursorRight),
        (false, false, tuinix::KeyCode::Delete) => act(Action::CharDeleteForward),
        (false, false, tuinix::KeyCode::Backspace) => act(Action::CharDeleteBackward),
        (false, false, tuinix::KeyCode::Enter) => act(Action::NewlineInsert),
        (false, false, tuinix::KeyCode::Up) => act(Action::CursorUp),
        (false, false, tuinix::KeyCode::Down) => act(Action::CursorDown),
        (false, false, tuinix::KeyCode::Left) => act(Action::CursorLeft),
        (false, false, tuinix::KeyCode::Right) => act(Action::CursorRight),
        (true, false, tuinix::KeyCode::Left) => act(Action::CursorUp),
        (true, false, tuinix::KeyCode::Right) => act(Action::CursorDown),
        // Any other bare, non-control character is text, not a binding.
        (false, false, tuinix::KeyCode::Char(ch)) if !ch.is_control() => act(Action::CharInsert),
        _ => return None,
    })
}

fn resolve_grep(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, alt, code } = *key;

    Some(match (ctrl, alt, code) {
        (true, false, tuinix::KeyCode::Char('g')) => cancel(),
        (false, false, tuinix::KeyCode::Enter) => cancel(),
        (true, false, tuinix::KeyCode::Char('y')) => act(Action::ClipboardPaste),
        (true, false, tuinix::KeyCode::Char('s')) => act(Action::GrepNextHit),
        (true, false, tuinix::KeyCode::Char('r')) => act(Action::GrepPrevHit),
        (false, false, tuinix::KeyCode::Tab) => act(Action::GrepNextHit),
        (false, false, tuinix::KeyCode::BackTab) => act(Action::GrepPrevHit),
        (true, false, tuinix::KeyCode::Char('a')) => act(Action::CursorLineStart),
        (true, false, tuinix::KeyCode::Char('e')) => act(Action::CursorLineEnd),
        (true, false, tuinix::KeyCode::Char('d')) => act(Action::CharDeleteForward),
        (true, false, tuinix::KeyCode::Char('h')) => act(Action::CharDeleteBackward),
        (true, false, tuinix::KeyCode::Char('b')) => act(Action::CursorLeft),
        (true, false, tuinix::KeyCode::Char('f')) => act(Action::CursorRight),
        (false, false, tuinix::KeyCode::Delete) => act(Action::CharDeleteForward),
        (false, false, tuinix::KeyCode::Backspace) => act(Action::CharDeleteBackward),
        (false, false, tuinix::KeyCode::Left) => act(Action::CursorLeft),
        (false, false, tuinix::KeyCode::Right) => act(Action::CursorRight),
        // Any other bare, non-control character is part of the query.
        (false, false, tuinix::KeyCode::Char(ch)) if !ch.is_control() => act(Action::CharInsert),
        _ => return None,
    })
}

fn resolve_ext(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, alt, code } = *key;

    Some(match (ctrl, alt, code) {
        (true, false, tuinix::KeyCode::Char('g')) => cancel(),
        (true, false, tuinix::KeyCode::Char('s')) => then(Action::BufferSave, Context::Main),
        _ => return None,
    })
}
