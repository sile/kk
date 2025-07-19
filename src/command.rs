use nojson::RawJsonValue;

#[derive(Debug, Clone)]
pub enum Command {
    Move(MoveCommand),
}

impl Command {
    pub fn parse(value: nojson::RawJsonValue<'_, '_>) -> Result<Self, nojson::JsonParseError> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "move" => MoveCommand::parse(value).map(Self::Move),
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoveCommand {
    pub row: isize,
    pub col: isize,
}

impl MoveCommand {
    fn parse(value: nojson::RawJsonValue<'_, '_>) -> Result<Self, nojson::JsonParseError> {
        Ok(Self {
            row: value.to_member("row")?.map(parse)?.unwrap_or(0),
            col: value.to_member("col")?.map(parse)?.unwrap_or(0),
        })
    }
}

fn parse<'text, 'raw, T>(value: RawJsonValue<'text, 'raw>) -> Result<T, nojson::JsonParseError>
where
    T: TryFrom<RawJsonValue<'text, 'raw>, Error = nojson::JsonParseError>,
{
    T::try_from(value)
}
