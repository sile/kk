pub type ActionName = String;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Cancel,
    BufferSave,
    BufferReload,
    BufferUndo,
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    CursorLineStart,
    CursorLineEnd,
    CursorBufferStart,
    CursorBufferEnd,
    NewlineInsert,
    CharInsert,
    CharDeleteBackward,
    CharDeleteForward,
    MarkSet,
    MarkCopy,
    MarkCut,
    ClipboardPaste,
    ShellCommand(ExternalCommandAction),
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "cancel" => Ok(Self::Cancel),
            "buffer-save" => Ok(Self::BufferSave),
            "buffer-reload" => Ok(Self::BufferReload),
            "buffer-undo" => Ok(Self::BufferUndo),
            "cursor-up" => Ok(Self::CursorUp),
            "cursor-down" => Ok(Self::CursorDown),
            "cursor-left" => Ok(Self::CursorLeft),
            "cursor-right" => Ok(Self::CursorRight),
            "cursor-line-start" => Ok(Self::CursorLineStart),
            "cursor-line-end" => Ok(Self::CursorLineEnd),
            "cursor-buffer-start" => Ok(Self::CursorBufferStart),
            "cursor-buffer-end" => Ok(Self::CursorBufferEnd),
            "newline-insert" => Ok(Self::NewlineInsert),
            "char-insert" => Ok(Self::CharInsert),
            "char-delete-backward" => Ok(Self::CharDeleteBackward),
            "char-delete-forward" => Ok(Self::CharDeleteForward),
            "mark-set" => Ok(Self::MarkSet),
            "mark-copy" => Ok(Self::MarkCopy),
            "mark-cut" => Ok(Self::MarkCut),
            "clipboard-paste" => Ok(Self::ClipboardPaste),
            "external-command" => ExternalCommandAction::try_from(value).map(Self::ShellCommand),
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExternalCommandAction {
    pub command: String,
    pub args: Vec<String>,
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
