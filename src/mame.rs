use tuinix::{KeyCode, KeyInput};

pub type TerminalFrame = tuinix::TerminalFrame<UnicodeCharWidthEstimator>;

#[derive(Debug, Default)]
pub struct UnicodeCharWidthEstimator;

impl tuinix::EstimateCharWidth for UnicodeCharWidthEstimator {
    fn estimate_char_width(&self, c: char) -> usize {
        unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyPattern {
    Literal(KeyInput),
    AlphaNumeric,
}

impl KeyPattern {
    pub fn matches(self, key: KeyInput) -> bool {
        match self {
            KeyPattern::Literal(k) => k == key,
            KeyPattern::AlphaNumeric => {
                if let KeyInput {
                    ctrl: false,
                    alt: false,
                    code: KeyCode::Char(ch),
                } = key
                {
                    ch.is_alphanumeric()
                } else {
                    false
                }
            }
        }
    }
}

impl std::str::FromStr for KeyPattern {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "<ALPHA_NUMERIC>" {
            return Ok(KeyPattern::AlphaNumeric);
        }

        // Handle special keys in angle brackets
        let special_key = |code| KeyInput {
            ctrl: false,
            alt: false,
            code,
        };
        match s {
            "<UP>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Up))),
            "<DOWN>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Down))),
            "<LEFT>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Left))),
            "<RIGHT>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Right))),
            "<ENTER>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Enter))),
            "<ESCAPE>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Escape))),
            "<BACKSPACE>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Backspace))),
            "<TAB>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Tab))),
            "<BACK_TAB>" => return Ok(KeyPattern::Literal(special_key(KeyCode::BackTab))),
            "<DELETE>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Delete))),
            "<INSERT>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Insert))),
            "<HOME>" => return Ok(KeyPattern::Literal(special_key(KeyCode::Home))),
            "<END>" => return Ok(KeyPattern::Literal(special_key(KeyCode::End))),
            "<PAGE_UP>" => return Ok(KeyPattern::Literal(special_key(KeyCode::PageUp))),
            "<PAGE_DOWN>" => return Ok(KeyPattern::Literal(special_key(KeyCode::PageDown))),
            _ => {}
        }

        // Handle modifier key combinations like "C-c", "M-x"
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

        // Handle character input
        let mut chars = remaining.chars();
        if let Some(ch) = chars.next()
            && chars.next().is_none()
        {
            let code = KeyCode::Char(ch);
            Ok(KeyPattern::Literal(KeyInput { ctrl, alt, code }))
        } else {
            Err(format!("invalid key input format: {s:?}"))
        }
    }
}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for KeyPattern {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        value
            .to_unquoted_string_str()?
            .parse()
            .map_err(|e| value.invalid(e))
    }
}

impl std::fmt::Display for KeyPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlphaNumeric => write!(f, "<ALPHA_NUMERIC>"),
            Self::Literal(key) => match key.code {
                KeyCode::Up => write!(f, "<UP>"),
                KeyCode::Down => write!(f, "<DOWN>"),
                KeyCode::Left => write!(f, "<LEFT>"),
                KeyCode::Right => write!(f, "<RIGHT>"),
                KeyCode::Enter => write!(f, "<ENTER>"),
                KeyCode::Escape => write!(f, "<ESCAPE>"),
                KeyCode::Backspace => write!(f, "<BACKSPACE>"),
                KeyCode::Tab => write!(f, "<TAB>"),
                KeyCode::BackTab => write!(f, "<BACK_TAB>"),
                KeyCode::Delete => write!(f, "<DELETE>"),
                KeyCode::Insert => write!(f, "<INSERT>"),
                KeyCode::Home => write!(f, "<HOME>"),
                KeyCode::End => write!(f, "<END>"),
                KeyCode::PageUp => write!(f, "<PAGE_UP>"),
                KeyCode::PageDown => write!(f, "<PAGE_DOWN>"),
                KeyCode::Char(ch) => {
                    if key.alt {
                        write!(f, "M-")?;
                    }
                    if key.ctrl {
                        write!(f, "C-")?;
                    }
                    write!(f, "{ch}")
                }
            },
        }
    }
}
