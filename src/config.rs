use std::collections::BTreeMap;

use tuinix::KeyInput;

use crate::{action::Action, mame};

#[derive(Debug, Clone)]
pub struct Config {
    pub commands: BTreeMap<String, Action>,
    pub keylabels: BTreeMap<KeyInput, String>,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Config {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            commands: value.to_member("commands")?.required()?.try_into()?,
            keylabels: value
                .to_member("keylabels")?
                .map(|v| {
                    v.to_object()?
                        .map(|(k, v)| Ok((mame::parse_key_input(k)?, v.try_into()?)))
                        .collect()
                })?
                .unwrap_or_default(),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        let nojson::Json(config) = include_str!("../config.json").parse().expect("bug");
        config
    }
}
