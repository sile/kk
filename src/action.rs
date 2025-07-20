pub type ActionName = String;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Cancel,
    Move(MoveAction),
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "cancel" => Ok(Self::Cancel),
            "move" => MoveAction::parse(value).map(Self::Move),
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MoveAction {
    pub rows: isize,
    pub cols: isize,
}

impl MoveAction {
    fn parse(value: nojson::RawJsonValue<'_, '_>) -> Result<Self, nojson::JsonParseError> {
        Ok(Self {
            rows: value.to_member("rows")?.map(parse)?.unwrap_or(0),
            cols: value.to_member("cols")?.map(parse)?.unwrap_or(0),
        })
    }
}

fn parse<'text, 'raw, T>(
    value: nojson::RawJsonValue<'text, 'raw>,
) -> Result<T, nojson::JsonParseError>
where
    T: TryFrom<nojson::RawJsonValue<'text, 'raw>, Error = nojson::JsonParseError>,
{
    T::try_from(value)
}
