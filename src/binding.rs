//! Input contexts and the `match`-based resolvers that map input to actions.

use crate::action::{Action, EchoAction, GrepAction};

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
        (false, true, tuinix::KeyCode::Char('w')) => act(Action::MarkCopy),
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
        (true, false, tuinix::KeyCode::Up) => act(Action::CursorPageUp),
        (true, false, tuinix::KeyCode::Down) => act(Action::CursorPageDown),
        (false, false, tuinix::KeyCode::PageUp) => act(Action::CursorPageUp),
        (false, false, tuinix::KeyCode::PageDown) => act(Action::CursorPageDown),
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
        (true, false, tuinix::KeyCode::Char('s')) => then(
            Action::Multiple(vec![
                Action::BufferSave,
                Action::Cancel,
                Action::Echo(EchoAction {
                    message: "Saved!".to_owned(),
                }),
            ]),
            Context::Main,
        ),
        _ => return None,
    })
}
