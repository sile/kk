pub type ActionName = String;

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
    Grep(GrepAction),
    GrepNextHit,
    GrepPrevHit,
    GrepNextQuery,
    GrepPrevQuery,
    ContextSet(ContextSetAction),
    Echo(EchoAction),
    GotoLine,
    Multiple(Vec<Action>),
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        if value.kind().is_array() {
            return Ok(Self::Multiple(value.try_into()?));
        }

        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "cancel" => Ok(Self::Cancel),
            "buffer-save" => Ok(Self::BufferSave),
            "buffer-reload" => Ok(Self::BufferReload),
            "buffer-undo" => Ok(Self::BufferUndo),
            "cursor-anchor" => Ok(Self::CursorAnchor),
            "cursor-jump" => Ok(Self::CursorJump),
            "cursor-up" => Ok(Self::CursorUp),
            "cursor-down" => Ok(Self::CursorDown),
            "cursor-left" => Ok(Self::CursorLeft),
            "cursor-right" => Ok(Self::CursorRight),
            "cursor-line-start" => Ok(Self::CursorLineStart),
            "cursor-line-end" => Ok(Self::CursorLineEnd),
            "cursor-buffer-start" => Ok(Self::CursorBufferStart),
            "cursor-buffer-end" => Ok(Self::CursorBufferEnd),
            "cursor-page-up" => Ok(Self::CursorPageUp),
            "cursor-page-down" => Ok(Self::CursorPageDown),
            "cursor-skip-spaces" => Ok(Self::CursorSkipSpaces),
            "cursor-up-skip-spaces" => Ok(Self::CursorUpSkipSpaces),
            "cursor-down-skip-spaces" => Ok(Self::CursorDownSkipSpaces),
            "cursor-left-skip-chars" => SkipChars::try_from(value).map(Self::CursorLeftSkipChars),
            "cursor-right-skip-chars" => SkipChars::try_from(value).map(Self::CursorRightSkipChars),
            "view-recenter" => Ok(Self::ViewRecenter),
            "newline-insert" => Ok(Self::NewlineInsert),
            "char-insert" => Ok(Self::CharInsert),
            "char-delete-backward" => Ok(Self::CharDeleteBackward),
            "char-delete-forward" => Ok(Self::CharDeleteForward),
            "line-delete" => Ok(Self::LineDelete),
            "mark-set" => Ok(Self::MarkSet),
            "mark-copy" => Ok(Self::MarkCopy),
            "mark-cut" => Ok(Self::MarkCut),
            "clipboard-paste" => Ok(Self::ClipboardPaste),
            "context-set" => ContextSetAction::try_from(value).map(Self::ContextSet),
            "echo" => EchoAction::try_from(value).map(Self::Echo),
            "external-command" => ExternalCommandAction::try_from(value).map(Self::ShellCommand),
            "grep" => GrepAction::try_from(value).map(Self::Grep),
            "grep-next-hit" => Ok(Self::GrepNextHit),
            "grep-prev-hit" => Ok(Self::GrepPrevHit),
            "grep-next-query" => Ok(Self::GrepNextQuery),
            "grep-prev-query" => Ok(Self::GrepPrevQuery),
            "goto-line" => Ok(Self::GotoLine),
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExternalCommandArg {
    Literal(String),
    CurrentFile,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for ExternalCommandArg {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        if let Ok(var) = value.to_member("var") {
            let var = var.required()?;
            match var.to_unquoted_string_str()?.as_ref() {
                "CURRENT_FILE" => Ok(Self::CurrentFile),
                _ => Err(var.invalid("unknown var")),
            }
        } else {
            Ok(Self::Literal(value.try_into()?))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalCommandAction {
    pub command: String,
    pub args: Vec<ExternalCommandArg>,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for ExternalCommandAction {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            command: value.to_member("command")?.required()?.try_into()?,
            args: value
                .to_member("args")?
                .map(Vec::try_from)?
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct GrepAction {
    pub command: String,
    pub args: Vec<String>,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for GrepAction {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            command: value.to_member("command")?.required()?.try_into()?,
            args: value
                .to_member("args")?
                .map(Vec::try_from)?
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ContextSetAction {
    pub name: String,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for ContextSetAction {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.to_member("name")?.required()?.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EchoAction {
    pub message: String,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for EchoAction {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            message: value.to_member("message")?.required()?.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SkipChars {
    pub chars: String,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for SkipChars {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            chars: value.to_member("chars")?.required()?.try_into()?,
        })
    }
}
