pub type ActionName = String;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Cancel,
    BufferSave,
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    CursorLineStart,
    CursorLineEnd,
    CursorBufferStart,
    CursorBufferEnd,
    CharDeleteBackward,
    CharDeleteForward,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "cancel" => Ok(Self::Cancel),
            "buffer-save" => Ok(Self::BufferSave),
            "cursor-up" => Ok(Self::CursorUp),
            "cursor-down" => Ok(Self::CursorDown),
            "cursor-left" => Ok(Self::CursorLeft),
            "cursor-right" => Ok(Self::CursorRight),
            "cursor-line-start" => Ok(Self::CursorLineStart),
            "cursor-line-end" => Ok(Self::CursorLineEnd),
            "cursor-buffer-start" => Ok(Self::CursorBufferStart),
            "cursor-buffer-end" => Ok(Self::CursorBufferEnd),
            "char-delete-backward" => Ok(Self::CharDeleteBackward),
            "char-delete-forward" => Ok(Self::CharDeleteForward),
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
