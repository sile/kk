//! Hard-coded input binding definitions.

use crate::{
    action::{Action, EchoAction, ExternalCommandAction, ExternalCommandArg, GrepAction, SkipChars},
    binding::{Binding, Context, InputMatcher},
};

/// Maximum number of grep matches to request.
pub const MAX_GREP_LINES: usize = 100;

const IDENT_CHARS: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";

fn triggers(specs: &[&str]) -> Vec<InputMatcher> {
    specs
        .iter()
        .map(|s| s.parse().expect("invalid trigger spec"))
        .collect()
}

fn grep_action(forward: bool) -> Action {
    Action::Grep(GrepAction {
        command: "grep".to_owned(),
        args: vec![
            "-m".to_owned(),
            MAX_GREP_LINES.to_string(),
            "-bio".to_owned(),
        ],
        forward,
    })
}

fn mark_ident() -> Action {
    Action::Multiple(vec![
        Action::CursorLeftSkipChars(SkipChars {
            chars: IDENT_CHARS.to_owned(),
        }),
        Action::CursorRight,
        Action::MarkSet,
        Action::CursorRightSkipChars(SkipChars {
            chars: IDENT_CHARS.to_owned(),
        }),
    ])
}

fn format_buffer() -> Action {
    Action::Multiple(vec![
        Action::BufferSave,
        Action::ShellCommand(ExternalCommandAction {
            command: "rustfmt".to_owned(),
            args: vec![
                ExternalCommandArg::Literal("--edition".to_owned()),
                ExternalCommandArg::Literal("2024".to_owned()),
                ExternalCommandArg::CurrentFile,
            ],
        }),
        Action::BufferReload,
        Action::CursorSkipSpaces,
        Action::Cancel,
        Action::Echo(EchoAction {
            message: "Formatted!".to_owned(),
        }),
    ])
}

pub fn main_bindings() -> Vec<Binding> {
    vec![
        Binding {
            triggers: triggers(&["C-c"]),
            label: Some("C-c: quit"),
            action: Some(Action::Quit),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-g"]),
            label: Some("C-g: cancel"),
            action: Some(Action::Cancel),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["M-r"]),
            label: Some("M-r: reload"),
            action: Some(Action::BufferReload),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-/", "C-0x7f", "C-u"]),
            label: Some("C-/: undo"),
            action: Some(Action::BufferUndo),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-l"]),
            label: Some("C-l: recenter"),
            action: Some(Action::ViewRecenter),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-s"]),
            label: Some("C-s: grep"),
            action: Some(Action::Multiple(vec![Action::CursorAnchor, grep_action(true)])),
            context: Some(Context::Grep),
        },
        Binding {
            triggers: triggers(&["C-r"]),
            label: Some("C-r: rgrep"),
            action: Some(Action::Multiple(vec![
                Action::CursorAnchor,
                grep_action(false),
            ])),
            context: Some(Context::Grep),
        },
        Binding {
            triggers: triggers(&["C- ", "C-`"]),
            label: Some("C- : mark"),
            action: Some(Action::MarkSet),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-w"]),
            label: Some("C-w: cut"),
            action: Some(Action::MarkCut),
            context: None,
        },
        Binding {
            triggers: triggers(&["M-w"]),
            label: Some("M-w: copy"),
            action: Some(Action::MarkCopy),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-y"]),
            label: Some("C-y: paste"),
            action: Some(Action::ClipboardPaste),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-k"]),
            label: Some("C-k: kill-line"),
            action: Some(Action::LineDelete),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-x"]),
            label: Some("C-x: ext-mode"),
            action: None,
            context: Some(Context::Ext),
        },
        Binding {
            triggers: triggers(&["M-g"]),
            label: Some("M-g: goto-mode"),
            action: None,
            context: Some(Context::Goto),
        },
        Binding {
            triggers: triggers(&["M-,"]),
            label: Some("M-,: jump"),
            action: Some(Action::CursorJump),
            context: None,
        },
        Binding {
            triggers: triggers(&["M-m"]),
            label: Some("M-m: mark-ident"),
            action: Some(mark_ident()),
            context: None,
        },
        Binding {
            triggers: triggers(&["M-l"]),
            label: Some("M-l: mark-line"),
            action: Some(Action::Multiple(vec![
                Action::CursorLineStart,
                Action::MarkSet,
                Action::CursorLineEnd,
            ])),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-a"]),
            label: None,
            action: Some(Action::CursorLineStart),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-e"]),
            label: None,
            action: Some(Action::CursorLineEnd),
            context: None,
        },
        Binding {
            triggers: triggers(&["M-<"]),
            label: None,
            action: Some(Action::CursorBufferStart),
            context: None,
        },
        Binding {
            triggers: triggers(&["M->"]),
            label: None,
            action: Some(Action::CursorBufferEnd),
            context: None,
        },
        Binding {
            triggers: triggers(&["<DELETE>", "C-d"]),
            label: None,
            action: Some(Action::CharDeleteForward),
            context: None,
        },
        Binding {
            triggers: triggers(&["<BACKSPACE>", "C-h"]),
            label: None,
            action: Some(Action::CharDeleteBackward),
            context: None,
        },
        Binding {
            triggers: triggers(&["<ENTER>", "C-j"]),
            label: None,
            action: Some(Action::NewlineInsert),
            context: None,
        },
        Binding {
            triggers: triggers(&["<UP>", "C-p", "C-<LEFT>"]),
            label: None,
            action: Some(Action::CursorUp),
            context: None,
        },
        Binding {
            triggers: triggers(&["<DOWN>", "C-n", "C-<RIGHT>"]),
            label: None,
            action: Some(Action::CursorDown),
            context: None,
        },
        Binding {
            triggers: triggers(&["<LEFT>", "C-b"]),
            label: None,
            action: Some(Action::CursorLeft),
            context: None,
        },
        Binding {
            triggers: triggers(&["<RIGHT>", "C-f"]),
            label: None,
            action: Some(Action::CursorRight),
            context: None,
        },
        Binding {
            triggers: triggers(&["<TAB>"]),
            label: None,
            action: Some(format_buffer()),
            context: None,
        },
        Binding {
            triggers: triggers(&["<PRINTABLE>"]),
            label: None,
            action: Some(Action::CharInsert),
            context: None,
        },
        Binding {
            triggers: triggers(&["<PAGEUP>", "C-<UP>"]),
            label: None,
            action: Some(Action::CursorPageUp),
            context: None,
        },
        Binding {
            triggers: triggers(&["<PAGEDOWN>", "C-<DOWN>"]),
            label: None,
            action: Some(Action::CursorPageDown),
            context: None,
        },
    ]
}

pub fn grep_bindings() -> Vec<Binding> {
    vec![
        Binding {
            triggers: triggers(&["C-g", "<ENTER>"]),
            label: Some("C-g: cancel"),
            action: Some(Action::Cancel),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["C-y"]),
            label: Some("C-y: paste"),
            action: Some(Action::ClipboardPaste),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-s", "<TAB>"]),
            label: Some("C-s: next-hit"),
            action: Some(Action::GrepNextHit),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-r", "<BACKTAB>"]),
            label: Some("C-r: prev-hit"),
            action: Some(Action::GrepPrevHit),
            context: None,
        },
        Binding {
            triggers: triggers(&["M-m"]),
            label: Some("M-m: mark-hit"),
            action: Some(Action::Multiple(vec![
                Action::Cancel,
                Action::CursorLeftSkipChars(SkipChars {
                    chars: IDENT_CHARS.to_owned(),
                }),
                Action::CursorRight,
                Action::MarkSet,
                Action::CursorRightSkipChars(SkipChars {
                    chars: IDENT_CHARS.to_owned(),
                }),
                Action::MarkCopy,
                Action::CursorJump,
            ])),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["C-v"]),
            label: Some("C-v: replace-hit"),
            action: Some(Action::GrepReplaceHit),
            context: None,
        },
        Binding {
            triggers: triggers(&["<PRINTABLE>"]),
            label: None,
            action: Some(Action::CharInsert),
            context: None,
        },
        Binding {
            triggers: triggers(&["<DELETE>", "C-d"]),
            label: None,
            action: Some(Action::CharDeleteForward),
            context: None,
        },
        Binding {
            triggers: triggers(&["<BACKSPACE>", "C-h"]),
            label: None,
            action: Some(Action::CharDeleteBackward),
            context: None,
        },
        Binding {
            triggers: triggers(&["<LEFT>", "C-b"]),
            label: None,
            action: Some(Action::CursorLeft),
            context: None,
        },
        Binding {
            triggers: triggers(&["<RIGHT>", "C-f"]),
            label: None,
            action: Some(Action::CursorRight),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-a"]),
            label: None,
            action: Some(Action::CursorLineStart),
            context: None,
        },
        Binding {
            triggers: triggers(&["C-e"]),
            label: None,
            action: Some(Action::CursorLineEnd),
            context: None,
        },
        Binding {
            triggers: triggers(&["<UP>", "C-p"]),
            label: None,
            action: Some(Action::GrepPrevQuery),
            context: None,
        },
        Binding {
            triggers: triggers(&["<DOWN>", "C-n"]),
            label: None,
            action: Some(Action::GrepNextQuery),
            context: None,
        },
    ]
}

pub fn ext_bindings() -> Vec<Binding> {
    vec![
        Binding {
            triggers: triggers(&["C-g"]),
            label: Some("C-g: cancel"),
            action: Some(Action::Cancel),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["C-s"]),
            label: Some("C-s: save"),
            action: Some(Action::Multiple(vec![
                Action::BufferSave,
                Action::Cancel,
                Action::Echo(EchoAction {
                    message: "Saved!".to_owned(),
                }),
            ])),
            context: Some(Context::Main),
        },
    ]
}

pub fn goto_bindings() -> Vec<Binding> {
    vec![
        Binding {
            triggers: triggers(&["C-g"]),
            label: Some("C-g: cancel"),
            action: Some(Action::Cancel),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["g"]),
            label: Some("g: goto"),
            action: Some(Action::Multiple(vec![
                Action::GotoLine,
                Action::Cancel,
                Action::Echo(EchoAction {
                    message: "Moved!".to_owned(),
                }),
            ])),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["p"]),
            label: Some("p: up-same-level"),
            action: Some(Action::Multiple(vec![
                Action::CursorUpSkipSpaces,
                Action::Cancel,
            ])),
            context: Some(Context::Main),
        },
        Binding {
            triggers: triggers(&["n"]),
            label: Some("n: down-same-level"),
            action: Some(Action::Multiple(vec![
                Action::CursorDownSkipSpaces,
                Action::Cancel,
            ])),
            context: Some(Context::Main),
        },
    ]
}
