use crate::command::ExternalCommand;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Cancel,
    BufferSave,
    BufferReload,
    BufferUndo,
    CursorAnchor,
    CursorJump,
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    CursorLineStart,
    CursorLineEnd,
    CursorBufferStart,
    CursorBufferEnd,
    CursorPageUp,
    CursorPageDown,
    CursorSkipSpaces,
    CursorUpSkipSpaces,
    CursorDownSkipSpaces,
    CursorLeftSkipChars(SkipChars),
    CursorRightSkipChars(SkipChars),
    ViewRecenter,
    NewlineInsert,
    CharInsert,
    CharDeleteBackward,
    CharDeleteForward,
    LineDelete,
    MarkSet,
    MarkCopy,
    MarkCut,
    ClipboardPaste,
    ShellCommand(ExternalCommandAction),
    Command(ExternalCommand),
    Grep(GrepAction),
    GrepNextHit,
    GrepPrevHit,
    GrepNextQuery,
    GrepPrevQuery,
    GrepReplaceHit,
    Echo(EchoAction),
    GotoLine,
    Multiple(Vec<Action>),
}

#[derive(Debug, Clone)]
pub enum ExternalCommandArg {
    Literal(String),
    CurrentFile,
}

#[derive(Debug, Clone)]
pub struct ExternalCommandAction {
    pub command: String,
    pub args: Vec<ExternalCommandArg>,
}

#[derive(Debug, Clone)]
pub struct GrepAction {
    pub command: String,
    pub args: Vec<String>,
    pub forward: bool,
}

#[derive(Debug, Clone)]
pub struct EchoAction {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SkipChars {
    pub chars: String,
}
