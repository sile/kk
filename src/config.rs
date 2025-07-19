use std::collections::BTreeMap;

use crate::action::Action;

#[derive(Debug, Clone)]
pub struct Config {
    pub commands: BTreeMap<String, Action>,
}
