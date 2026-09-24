//! Formatting utilities for terminal UI display elements.

use crate::binding::InputMatcher;

/// Creates a displayable representation of a terminal input (key or mouse).
pub fn input(input: &tuinix::Input) -> String {
    match input {
        tuinix::Input::Key(key) => InputMatcher::Key(*key).to_string(),
        tuinix::Input::Mouse(mouse) => InputMatcher::Mouse(mouse.kind).to_string(),
        tuinix::Input::Unrecognized { .. } => "<UNRECOGNIZED>".to_owned(),
        tuinix::Input::Paste { .. } => "<PASTE>".to_owned(),
    }
}
