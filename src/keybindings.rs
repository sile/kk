use std::collections::BTreeMap;

use orfail::OrFail;
use tuinix::KeyInput;

use crate::{action::ActionName, mame::KeyPattern};

#[derive(Debug)]
pub struct KeybindingsContext {
    stack: Vec<String>,
}

impl KeybindingsContext {
    pub fn current_group_name(&self) -> &str {
        self.stack.last().expect("bug")
    }

    pub fn group_path(&self) -> String {
        self.stack.join(".")
    }

    pub fn enter(&mut self, name: &str) {
        self.stack.clear();
        self.stack.push(name.to_owned());
    }
}

impl Default for KeybindingsContext {
    fn default() -> Self {
        Self {
            stack: vec!["__main__".to_owned()],
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
        // TODO: remove Result (add validation when config JSON parsing instead)
        let group = self.groups.get(context.current_group_name()).or_fail()?;
        Ok(group.entries.iter())
    }

    pub fn get(&self, context: &KeybindingsContext, key: KeyInput) -> Option<&ActionName> {
        self.groups
            .get(context.current_group_name())?
            .entries
            .iter()
            .find_map(|b| b.key.matches(key).then_some(&b.action))
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
            match key.to_unquoted_string_str()?.as_ref() {
                "__hidden__" => {
                    for (key, value) in value.to_object()? {
                        entries.push(Keybinding {
                            key: key.try_into()?,
                            action: value.try_into()?,
                            visible: false,
                        });
                    }
                    continue;
                }
                "__comment__" => {
                    continue;
                }
                _ => {}
            }

            entries.push(Keybinding {
                key: key.try_into()?,
                action: value.try_into()?,
                visible: true,
            });
        }
        Ok(Self { entries })
    }
}

#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: KeyPattern,
    pub action: ActionName,
    pub visible: bool,
}
