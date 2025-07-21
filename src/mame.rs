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
    VisibleChars,
}

impl KeyPattern {
    pub fn matches(self, key: KeyInput) -> bool {
        match self {
            KeyPattern::Literal(k) => k == key,
            KeyPattern::VisibleChars => {
                if let KeyInput {
                    ctrl: false,
                    alt: false,
                    code: KeyCode::Char(ch),
                } = key
                {
                    !ch.is_control()
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
        if s == "<VISIBLE>" {
            return Ok(KeyPattern::VisibleChars);
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

        // Handle special keys in angle brackets
        let key = |code| KeyInput { ctrl, alt, code };
        match remaining {
            "<UP>" => return Ok(KeyPattern::Literal(key(KeyCode::Up))),
            "<DOWN>" => return Ok(KeyPattern::Literal(key(KeyCode::Down))),
            "<LEFT>" => return Ok(KeyPattern::Literal(key(KeyCode::Left))),
            "<RIGHT>" => return Ok(KeyPattern::Literal(key(KeyCode::Right))),
            "<ENTER>" => return Ok(KeyPattern::Literal(key(KeyCode::Enter))),
            "<ESCAPE>" => return Ok(KeyPattern::Literal(key(KeyCode::Escape))),
            "<BACKSPACE>" => return Ok(KeyPattern::Literal(key(KeyCode::Backspace))),
            "<TAB>" => return Ok(KeyPattern::Literal(key(KeyCode::Tab))),
            "<BACK_TAB>" => return Ok(KeyPattern::Literal(key(KeyCode::BackTab))),
            "<DELETE>" => return Ok(KeyPattern::Literal(key(KeyCode::Delete))),
            "<INSERT>" => return Ok(KeyPattern::Literal(key(KeyCode::Insert))),
            "<HOME>" => return Ok(KeyPattern::Literal(key(KeyCode::Home))),
            "<END>" => return Ok(KeyPattern::Literal(key(KeyCode::End))),
            "<PAGE_UP>" => return Ok(KeyPattern::Literal(key(KeyCode::PageUp))),
            "<PAGE_DOWN>" => return Ok(KeyPattern::Literal(key(KeyCode::PageDown))),
            _ => {}
        }

        // Handle character input
        let mut chars = remaining.chars();
        if let Some(ch) = chars.next()
            && chars.next().is_none()
        {
            let code = KeyCode::Char(ch);
            Ok(KeyPattern::Literal(key(code)))
        } else if let Some(hex_str) = remaining.strip_prefix("0x") {
            // Handle hex notation for control chars such as 0x7f
            match u32::from_str_radix(hex_str, 16) {
                Ok(code_point) => {
                    if let Some(ch) = char::from_u32(code_point) {
                        let code = KeyCode::Char(ch);
                        Ok(KeyPattern::Literal(key(code)))
                    } else {
                        Err(format!("invalid Unicode code point: 0x{:x}", code_point))
                    }
                }
                Err(_) => Err(format!("invalid hex notation: {}", remaining)),
            }
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
            Self::VisibleChars => write!(f, "<VISIBLE>"),
            Self::Literal(key) => {
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
                    KeyCode::BackTab => write!(f, "<BACK_TAB>"),
                    KeyCode::Delete => write!(f, "<DELETE>"),
                    KeyCode::Insert => write!(f, "<INSERT>"),
                    KeyCode::Home => write!(f, "<HOME>"),
                    KeyCode::End => write!(f, "<END>"),
                    KeyCode::PageUp => write!(f, "<PAGE_UP>"),
                    KeyCode::PageDown => write!(f, "<PAGE_DOWN>"),
                    KeyCode::Char(ch) if ch.is_control() => write!(f, "0x{:x}", ch as u32),
                    KeyCode::Char(ch) => write!(f, "{ch}"),
                }
            }
        }
    }
}
