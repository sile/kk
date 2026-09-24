//! Hard-coded input bindings.

use crate::action::Action;

/// Identifies one of the built-in input contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Context {
    /// The default editing context.
    Main,

    /// The context active while grep mode is collecting a query.
    Grep,

    /// A context reserved for extensions.
    Ext,

    /// The context active while a go-to line prompt is collecting input.
    Goto,
}

/// Matches terminal input against a specific pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMatcher {
    /// Matches an exact key combination.
    Key(tuinix::KeyInput),

    /// Matches any printable character.
    Printable,

    /// Matches a specific mouse event (display / lookup support).
    Mouse(tuinix::MouseInputKind),
}

impl InputMatcher {
    /// Returns `true` if the given terminal input matches this matcher.
    pub fn matches(self, input: &tuinix::Input) -> bool {
        match input {
            tuinix::Input::Key(key) => match self {
                InputMatcher::Key(k) => k == *key,
                InputMatcher::Printable => {
                    matches!(key, tuinix::KeyInput {
                        ctrl: false,
                        alt: false,
                        code: tuinix::KeyCode::Char(ch),
                    } if !ch.is_control())
                }
                InputMatcher::Mouse(_) => false,
            },
            tuinix::Input::Mouse(m) => {
                matches!(self, InputMatcher::Mouse(kind) if kind == m.kind)
            }
            tuinix::Input::Unrecognized { .. } | tuinix::Input::Paste { .. } => false,
        }
    }
}

impl std::str::FromStr for InputMatcher {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle inputs that do not accept modifiers.
        let mouse = |kind| InputMatcher::Mouse(kind);
        match s {
            "<PRINTABLE>" => return Ok(InputMatcher::Printable),
            "<LEFTCLICK>" => return Ok(mouse(tuinix::MouseInputKind::LeftPress)),
            "<LEFTRELEASE>" => return Ok(mouse(tuinix::MouseInputKind::LeftRelease)),
            "<RIGHTCLICK>" => return Ok(mouse(tuinix::MouseInputKind::RightPress)),
            "<RIGHTRELEASE>" => return Ok(mouse(tuinix::MouseInputKind::RightRelease)),
            "<MIDDLECLICK>" => return Ok(mouse(tuinix::MouseInputKind::MiddlePress)),
            "<MIDDLERELEASE>" => return Ok(mouse(tuinix::MouseInputKind::MiddleRelease)),
            "<DRAG>" => return Ok(mouse(tuinix::MouseInputKind::Drag)),
            "<SCROLLUP>" => return Ok(mouse(tuinix::MouseInputKind::ScrollUp)),
            "<SCROLLDOWN>" => return Ok(mouse(tuinix::MouseInputKind::ScrollDown)),
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
        let key = |code| InputMatcher::Key(tuinix::KeyInput { ctrl, alt, code });
        match remaining {
            "<UP>" => return Ok(key(tuinix::KeyCode::Up)),
            "<DOWN>" => return Ok(key(tuinix::KeyCode::Down)),
            "<LEFT>" => return Ok(key(tuinix::KeyCode::Left)),
            "<RIGHT>" => return Ok(key(tuinix::KeyCode::Right)),
            "<ENTER>" => return Ok(key(tuinix::KeyCode::Enter)),
            "<ESCAPE>" => return Ok(key(tuinix::KeyCode::Escape)),
            "<BACKSPACE>" => return Ok(key(tuinix::KeyCode::Backspace)),
            "<TAB>" => return Ok(key(tuinix::KeyCode::Tab)),
            "<BACKTAB>" => return Ok(key(tuinix::KeyCode::BackTab)),
            "<DELETE>" => return Ok(key(tuinix::KeyCode::Delete)),
            "<INSERT>" => return Ok(key(tuinix::KeyCode::Insert)),
            "<HOME>" => return Ok(key(tuinix::KeyCode::Home)),
            "<END>" => return Ok(key(tuinix::KeyCode::End)),
            "<PAGEUP>" => return Ok(key(tuinix::KeyCode::PageUp)),
            "<PAGEDOWN>" => return Ok(key(tuinix::KeyCode::PageDown)),
            _ => {}
        }

        // Handle character input.
        let mut chars = remaining.chars();
        if let Some(ch) = chars.next()
            && chars.next().is_none()
        {
            return Ok(key(tuinix::KeyCode::Char(ch)));
        }

        // Handle hex notation for control chars such as 0x7f.
        if let Some(hex_str) = remaining.strip_prefix("0x") {
            return match u32::from_str_radix(hex_str, 16) {
                Ok(code_point) => match char::from_u32(code_point) {
                    Some(ch) => Ok(key(tuinix::KeyCode::Char(ch))),
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
                    tuinix::KeyCode::Up => write!(f, "<UP>"),
                    tuinix::KeyCode::Down => write!(f, "<DOWN>"),
                    tuinix::KeyCode::Left => write!(f, "<LEFT>"),
                    tuinix::KeyCode::Right => write!(f, "<RIGHT>"),
                    tuinix::KeyCode::Enter => write!(f, "<ENTER>"),
                    tuinix::KeyCode::Escape => write!(f, "<ESCAPE>"),
                    tuinix::KeyCode::Backspace => write!(f, "<BACKSPACE>"),
                    tuinix::KeyCode::Tab => write!(f, "<TAB>"),
                    tuinix::KeyCode::BackTab => write!(f, "<BACKTAB>"),
                    tuinix::KeyCode::Delete => write!(f, "<DELETE>"),
                    tuinix::KeyCode::Insert => write!(f, "<INSERT>"),
                    tuinix::KeyCode::Home => write!(f, "<HOME>"),
                    tuinix::KeyCode::End => write!(f, "<END>"),
                    tuinix::KeyCode::PageUp => write!(f, "<PAGEUP>"),
                    tuinix::KeyCode::PageDown => write!(f, "<PAGEDOWN>"),
                    tuinix::KeyCode::F(n) => write!(f, "<F{n}>"),
                    tuinix::KeyCode::Char(ch) if ch.is_control() => write!(f, "0x{:x}", ch as u32),
                    tuinix::KeyCode::Char(ch) => write!(f, "{ch}"),
                }
            }
            Self::Mouse(mouse) => match mouse {
                tuinix::MouseInputKind::LeftPress => write!(f, "<LEFTCLICK>"),
                tuinix::MouseInputKind::LeftRelease => write!(f, "<LEFTRELEASE>"),
                tuinix::MouseInputKind::RightPress => write!(f, "<RIGHTCLICK>"),
                tuinix::MouseInputKind::RightRelease => write!(f, "<RIGHTRELEASE>"),
                tuinix::MouseInputKind::MiddlePress => write!(f, "<MIDDLECLICK>"),
                tuinix::MouseInputKind::MiddleRelease => write!(f, "<MIDDLERELEASE>"),
                tuinix::MouseInputKind::Drag => write!(f, "<DRAG>"),
                tuinix::MouseInputKind::ScrollUp => write!(f, "<SCROLLUP>"),
                tuinix::MouseInputKind::ScrollDown => write!(f, "<SCROLLDOWN>"),
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
    pub fn matches(&self, input: &tuinix::Input) -> bool {
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
