//! Formatting utilities for terminal UI display elements.

use crate::binding::InputMatcher;

/// Creates a displayable representation of a terminal input (key or mouse).
pub fn input(input: tuinix::TerminalInput) -> impl std::fmt::Display {
    match input {
        tuinix::TerminalInput::Key(key) => InputMatcher::Key(key),
        tuinix::TerminalInput::Mouse(mouse) => InputMatcher::Mouse(mouse.event),
    }
}
