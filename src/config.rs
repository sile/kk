use std::collections::BTreeMap;

use tuinix::KeyInput;

use crate::{
    action::{Action, ActionName},
    keybindings::Keybindings,
    mame,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub actions: BTreeMap<ActionName, Action>,
    pub keylabels: BTreeMap<KeyInput, String>,
    pub keybindings: Keybindings,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Config {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            actions: value.to_member("actions")?.required()?.try_into()?,
            keylabels: value
                .to_member("keylabels")?
                .required()?
                .to_object()?
                .map(|(k, v)| Ok((mame::parse_key_input(k)?, String::try_from(v)?)))
                .collect::<Result<_, _>>()?,
            keybindings: value.to_member("keybindings")?.required()?.try_into()?,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        let nojson::Json(config) = include_str!("../config.json").parse().expect("bug");
        config
    }
}
