use std::collections::BTreeMap;

use crate::command::Command;

#[derive(Debug, Clone)]
pub struct Config {
    pub commands: BTreeMap<String, Command>,
}
