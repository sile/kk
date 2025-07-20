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

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for KeyPattern {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let key_str = value.to_unquoted_string_str()?;

        if key_str.as_ref() == "<ALPHA_NUMERIC>" {
            return Ok(KeyPattern::AlphaNumeric);
        }

        let key_input = parse_key_input(value)?;
        Ok(KeyPattern::Literal(key_input))
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

// TODO: remove
pub fn parse_key_input(
    value: nojson::RawJsonValue<'_, '_>,
) -> Result<tuinix::KeyInput, nojson::JsonParseError> {
    let key_str = value.to_unquoted_string_str()?;

    // Handle special keys in angle brackets
    let special_key = |code| tuinix::KeyInput {
        ctrl: false,
        alt: false,
        code,
    };
    match key_str.as_ref() {
        "<UP>" => return Ok(special_key(KeyCode::Up)),
        "<DOWN>" => return Ok(special_key(KeyCode::Down)),
        "<LEFT>" => return Ok(special_key(KeyCode::Left)),
        "<RIGHT>" => return Ok(special_key(KeyCode::Right)),
        "<ENTER>" => return Ok(special_key(KeyCode::Enter)),
        "<ESCAPE>" => return Ok(special_key(KeyCode::Escape)),
        "<BACKSPACE>" => return Ok(special_key(KeyCode::Backspace)),
        "<TAB>" => return Ok(special_key(KeyCode::Tab)),
        "<BACK_TAB>" => return Ok(special_key(KeyCode::BackTab)),
        "<DELETE>" => return Ok(special_key(KeyCode::Delete)),
        "<INSERT>" => return Ok(special_key(KeyCode::Insert)),
        "<HOME>" => return Ok(special_key(KeyCode::Home)),
        "<END>" => return Ok(special_key(KeyCode::End)),
        "<PAGE_UP>" => return Ok(special_key(KeyCode::PageUp)),
        "<PAGE_DOWN>" => return Ok(special_key(KeyCode::PageDown)),
        _ => {}
    }

    // Handle modifier key combinations like "C-c", "M-x"
    let mut alt = false;
    let mut ctrl = false;
    let mut s = key_str.as_ref();
    loop {
        if let Some(remaining) = s.strip_prefix("M-")
            && !alt
        {
            s = remaining;
            alt = true;
        } else if let Some(remaining) = s.strip_prefix("C-")
            && !ctrl
        {
            s = remaining;
            ctrl = true;
        } else {
            break;
        }
    }

    // Handle character input
    let mut chars = s.chars();
    if let Some(ch) = chars.next()
        && None == chars.next()
    {
        let code = KeyCode::Char(ch);
        Ok(tuinix::KeyInput { ctrl, alt, code })
    } else {
        Err(value.invalid(format!("invalid key input format: {key_str:?}")))
    }
}

#[derive(Debug)]
pub struct KeyInputDisplay(pub tuinix::KeyInput);

impl std::fmt::Display for KeyInputDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_input = self.0;

        // Handle special keys first
        match key_input.code {
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
                // Handle modifier prefixes
                if key_input.alt {
                    write!(f, "M-")?;
                }
                if key_input.ctrl {
                    write!(f, "C-")?;
                }
                write!(f, "{}", ch)
            }
        }
    }
}
