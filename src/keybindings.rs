use std::collections::BTreeMap;

use orfail::OrFail;
use tuinix::KeyInput;

use crate::action::ActionName;

#[derive(Debug)]
pub struct KeybindingsContext {
    pub current_group_name: String,
    pub next_group_name: String,
}

impl Default for KeybindingsContext {
    fn default() -> Self {
        Self {
            current_group_name: "__main__".to_owned(),
            next_group_name: "__main__".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keybindings {
    pub groups: BTreeMap<String, KeybindingsGroup>,
}

impl Keybindings {
    pub fn iter(
        &self,
        context: &KeybindingsContext,
    ) -> orfail::Result<impl Iterator<Item = &Keybinding>> {
        let group = self.groups.get(&context.current_group_name).or_fail()?;
        Ok(group.entries.iter())
    }
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Keybindings {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let groups: BTreeMap<String, KeybindingsGroup> = value.try_into()?;
        Ok(Self { groups })
    }
}

#[derive(Debug, Clone)]
pub struct KeybindingsGroup {
    pub entries: Vec<Keybinding>,
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for KeybindingsGroup {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let mut entries = Vec::new();
        for (key, value) in value.to_object()? {
            if key.as_raw_str() == "\"__hidden__\"" {
                for (key, value) in value.to_object()? {
                    entries.push(Keybinding {
                        key: crate::mame::parse_key_input(key)?,
                        action: value.try_into()?,
                        visible: false,
                    });
                }
                continue;
            }

            entries.push(Keybinding {
                key: crate::mame::parse_key_input(key)?,
                action: value.try_into()?,
                visible: true,
            });
        }
        Ok(Self { entries })
    }
}

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: KeyInput,
    pub action: ActionName,
    pub visible: bool,
}
