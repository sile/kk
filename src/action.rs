use tuinix::KeyInput;

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Move(MoveAction),
    InputChar(InputCharAction),
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "move" => MoveAction::parse(value).map(Self::Move),
            "input_char" => InputCharAction::parse(value).map(Self::InputChar),
            ty => Err(value.invalid(format!("unknown command type: {ty:?}"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MoveAction {
    pub row: isize,
    pub col: isize,
}

impl MoveAction {
    fn parse(value: nojson::RawJsonValue<'_, '_>) -> Result<Self, nojson::JsonParseError> {
        Ok(Self {
            row: value.to_member("row")?.map(parse)?.unwrap_or(0),
            col: value.to_member("col")?.map(parse)?.unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone)]
pub struct InputCharAction {
    pub value: KeyInput,
}

impl InputCharAction {
    fn parse(_value: nojson::RawJsonValue<'_, '_>) -> Result<Self, nojson::JsonParseError> {
        // let value = value.to_member("value")?.required()?.try_into()?;
        // Ok(Self { value })
        todo!()
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
