use std::collections::BTreeMap;

use crate::action::Action;

#[derive(Debug, Clone)]
pub struct Config {
    pub commands: BTreeMap<String, Action>,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Config {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            commands: value.to_member("commands")?.required()?.try_into()?,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        let nojson::Json(config) = include_str!("../config.json").parse().expect("bug");
        config
    }
}
