//! Hard-coded input bindings.

use tuinix::{KeyCode, KeyInput};

use crate::action::Action;

/// Identifies one of the built-in input contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Context {
    Main,
    Grep,
    Ext,
    Goto,
}

/// Matches terminal input against a specific pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMatcher {
    /// Matches an exact key combination.
    Key(KeyInput),

    /// Matches any printable character.
    Printable,

    /// Matches a specific mouse event (display / lookup support).
    Mouse(tuinix::MouseEvent),
}

impl InputMatcher {
    /// Returns `true` if the given terminal input matches this matcher.
    pub fn matches(self, input: tuinix::TerminalInput) -> bool {
        match input {
            tuinix::TerminalInput::Key(key) => match self {
                InputMatcher::Key(k) => k == key,
                InputMatcher::Printable => {
                    matches!(key, KeyInput {
                        ctrl: false,
                        alt: false,
                        code: KeyCode::Char(ch),
                    } if !ch.is_control())
                }
                InputMatcher::Mouse(_) => false,
            },
            tuinix::TerminalInput::Mouse(m) => {
                matches!(self, InputMatcher::Mouse(e) if e == m.event)
            }
        }
    }
}

impl std::str::FromStr for InputMatcher {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle inputs that do not accept modifiers.
        let mouse = |event| InputMatcher::Mouse(event);
        match s {
            "<PRINTABLE>" => return Ok(InputMatcher::Printable),
            "<LEFTCLICK>" => return Ok(mouse(tuinix::MouseEvent::LeftPress)),
            "<LEFTRELEASE>" => return Ok(mouse(tuinix::MouseEvent::LeftRelease)),
            "<RIGHTCLICK>" => return Ok(mouse(tuinix::MouseEvent::RightPress)),
            "<RIGHTRELEASE>" => return Ok(mouse(tuinix::MouseEvent::RightRelease)),
            "<MIDDLECLICK>" => return Ok(mouse(tuinix::MouseEvent::MiddlePress)),
            "<MIDDLERELEASE>" => return Ok(mouse(tuinix::MouseEvent::MiddleRelease)),
            "<DRAG>" => return Ok(mouse(tuinix::MouseEvent::Drag)),
            "<SCROLLUP>" => return Ok(mouse(tuinix::MouseEvent::ScrollUp)),
            "<SCROLLDOWN>" => return Ok(mouse(tuinix::MouseEvent::ScrollDown)),
            _ => {}
        }

        // Handle modifier key combinations like "C-c", "M-x".
        let mut alt = false;
        let mut ctrl = false;
        let mut remaining = s;

        loop {
            if let Some(rest) = remaining.strip_prefix("M-")
                && !alt
            {
                remaining = rest;
                alt = true;
            } else if let Some(rest) = remaining.strip_prefix("C-")
                && !ctrl
            {
                remaining = rest;
                ctrl = true;
            } else {
                break;
            }
        }

        // Handle special keys.
        let key = |code| InputMatcher::Key(KeyInput { ctrl, alt, code });
        match remaining {
            "<UP>" => return Ok(key(KeyCode::Up)),
            "<DOWN>" => return Ok(key(KeyCode::Down)),
            "<LEFT>" => return Ok(key(KeyCode::Left)),
            "<RIGHT>" => return Ok(key(KeyCode::Right)),
            "<ENTER>" => return Ok(key(KeyCode::Enter)),
            "<ESCAPE>" => return Ok(key(KeyCode::Escape)),
            "<BACKSPACE>" => return Ok(key(KeyCode::Backspace)),
            "<TAB>" => return Ok(key(KeyCode::Tab)),
            "<BACKTAB>" => return Ok(key(KeyCode::BackTab)),
            "<DELETE>" => return Ok(key(KeyCode::Delete)),
            "<INSERT>" => return Ok(key(KeyCode::Insert)),
            "<HOME>" => return Ok(key(KeyCode::Home)),
            "<END>" => return Ok(key(KeyCode::End)),
            "<PAGEUP>" => return Ok(key(KeyCode::PageUp)),
            "<PAGEDOWN>" => return Ok(key(KeyCode::PageDown)),
            _ => {}
        }

        // Handle character input.
        let mut chars = remaining.chars();
        if let Some(ch) = chars.next()
            && chars.next().is_none()
        {
            return Ok(key(KeyCode::Char(ch)));
        }

        // Handle hex notation for control chars such as 0x7f.
        if let Some(hex_str) = remaining.strip_prefix("0x") {
            return match u32::from_str_radix(hex_str, 16) {
                Ok(code_point) => match char::from_u32(code_point) {
                    Some(ch) => Ok(key(KeyCode::Char(ch))),
                    None => Err(format!("invalid Unicode code point: 0x{code_point:x}")),
                },
                Err(_) => Err(format!("invalid hex notation: {remaining}")),
            };
        }

        Err(format!("invalid key input format: {s:?}"))
    }
}

impl std::fmt::Display for InputMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Printable => write!(f, "<PRINTABLE>"),
            Self::Key(key) => {
                if key.alt {
                    write!(f, "M-")?;
                }
                if key.ctrl {
                    write!(f, "C-")?;
                }

                match key.code {
                    KeyCode::Up => write!(f, "<UP>"),
                    KeyCode::Down => write!(f, "<DOWN>"),
                    KeyCode::Left => write!(f, "<LEFT>"),
                    KeyCode::Right => write!(f, "<RIGHT>"),
                    KeyCode::Enter => write!(f, "<ENTER>"),
                    KeyCode::Escape => write!(f, "<ESCAPE>"),
                    KeyCode::Backspace => write!(f, "<BACKSPACE>"),
                    KeyCode::Tab => write!(f, "<TAB>"),
                    KeyCode::BackTab => write!(f, "<BACKTAB>"),
                    KeyCode::Delete => write!(f, "<DELETE>"),
                    KeyCode::Insert => write!(f, "<INSERT>"),
                    KeyCode::Home => write!(f, "<HOME>"),
                    KeyCode::End => write!(f, "<END>"),
                    KeyCode::PageUp => write!(f, "<PAGEUP>"),
                    KeyCode::PageDown => write!(f, "<PAGEDOWN>"),
                    KeyCode::Char(ch) if ch.is_control() => write!(f, "0x{:x}", ch as u32),
                    KeyCode::Char(ch) => write!(f, "{ch}"),
                }
            }
            Self::Mouse(mouse) => match mouse {
                tuinix::MouseEvent::LeftPress => write!(f, "<LEFTCLICK>"),
                tuinix::MouseEvent::LeftRelease => write!(f, "<LEFTRELEASE>"),
                tuinix::MouseEvent::RightPress => write!(f, "<RIGHTCLICK>"),
                tuinix::MouseEvent::RightRelease => write!(f, "<RIGHTRELEASE>"),
                tuinix::MouseEvent::MiddlePress => write!(f, "<MIDDLECLICK>"),
                tuinix::MouseEvent::MiddleRelease => write!(f, "<MIDDLERELEASE>"),
                tuinix::MouseEvent::Drag => write!(f, "<DRAG>"),
                tuinix::MouseEvent::ScrollUp => write!(f, "<SCROLLUP>"),
                tuinix::MouseEvent::ScrollDown => write!(f, "<SCROLLDOWN>"),
            },
        }
    }
}

/// A single input binding that maps terminal input patterns to an action.
#[derive(Debug, Clone)]
pub struct Binding {
    /// Input patterns that trigger this binding.
    pub triggers: Vec<InputMatcher>,

    /// Optional human-readable label for display purposes.
    pub label: Option<&'static str>,

    /// Optional action to execute when the binding is triggered.
    pub action: Option<Action>,

    /// Optional context to switch to when this binding is activated.
    pub context: Option<Context>,
}

impl Binding {
    /// Checks if this binding matches the given terminal input.
    pub fn matches(&self, input: tuinix::TerminalInput) -> bool {
        self.triggers.iter().any(|t| t.matches(input))
    }
}

/// The set of hard-coded bindings, resolved at startup.
#[derive(Debug)]
pub struct Bindings {
    main: Vec<Binding>,
    grep: Vec<Binding>,
    ext: Vec<Binding>,
    goto: Vec<Binding>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self::new()
    }
}

impl Bindings {
    /// Builds the binding tables from the hard-coded definitions.
    pub fn new() -> Self {
        Self {
            main: crate::bindings::main_bindings(),
            grep: crate::bindings::grep_bindings(),
            ext: crate::bindings::ext_bindings(),
            goto: crate::bindings::goto_bindings(),
        }
    }

    /// Returns the bindings for the given context.
    pub fn get(&self, context: Context) -> &[Binding] {
        match context {
            Context::Main => &self.main,
            Context::Grep => &self.grep,
            Context::Ext => &self.ext,
            Context::Goto => &self.goto,
        }
    }
}
