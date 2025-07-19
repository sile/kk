pub type TerminalFrame = tuinix::TerminalFrame<UnicodeCharWidthEstimator>;

#[derive(Debug, Default)]
pub struct UnicodeCharWidthEstimator;

impl tuinix::EstimateCharWidth for UnicodeCharWidthEstimator {
    fn estimate_char_width(&self, c: char) -> usize {
        unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
    }
}

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
        "<UP>" => return Ok(special_key(tuinix::KeyCode::Up)),
        "<DOWN>" => return Ok(special_key(tuinix::KeyCode::Down)),
        "<LEFT>" => return Ok(special_key(tuinix::KeyCode::Left)),
        "<RIGHT>" => return Ok(special_key(tuinix::KeyCode::Right)),
        "<ENTER>" => return Ok(special_key(tuinix::KeyCode::Enter)),
        "<ESCAPE>" => return Ok(special_key(tuinix::KeyCode::Escape)),
        "<BACKSPACE>" => return Ok(special_key(tuinix::KeyCode::Backspace)),
        "<TAB>" => return Ok(special_key(tuinix::KeyCode::Tab)),
        "<DELETE>" => return Ok(special_key(tuinix::KeyCode::Delete)),
        "<INSERT>" => return Ok(special_key(tuinix::KeyCode::Insert)),
        "<HOME>" => return Ok(special_key(tuinix::KeyCode::Home)),
        "<END>" => return Ok(special_key(tuinix::KeyCode::End)),
        "<PAGE_UP>" => return Ok(special_key(tuinix::KeyCode::PageUp)),
        "<PAGE_DOWN>" => return Ok(special_key(tuinix::KeyCode::PageDown)),
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
        let code = tuinix::KeyCode::Char(ch);
        Ok(tuinix::KeyInput { ctrl, alt, code })
    } else {
        Err(value.invalid(format!("invalid key input format: {key_str:?}")))
    }
}
