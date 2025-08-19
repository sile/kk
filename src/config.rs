use std::collections::BTreeMap;

use mame::KeyMatcher as KeyPattern;

use crate::{
    action::{Action, ActionName},
    keybindings::Keybindings,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub actions: BTreeMap<ActionName, Action>,
    pub keylabels: BTreeMap<KeyPattern, String>,
    pub keybindings: Keybindings,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Config {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        Ok(Self {
            actions: value.to_member("actions")?.required()?.try_into()?,
            keylabels: value.to_member("keylabels")?.required()?.try_into()?,
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
