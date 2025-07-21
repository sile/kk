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
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

// fn parse<'text, 'raw, T>(
//     value: nojson::RawJsonValue<'text, 'raw>,
// ) -> Result<T, nojson::JsonParseError>
// where
//     T: TryFrom<nojson::RawJsonValue<'text, 'raw>, Error = nojson::JsonParseError>,
// {
//     T::try_from(value)
// }
