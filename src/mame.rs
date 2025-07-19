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

    // Handle modifier key combinations like "C-c", "A-x", "M-x"
    if key_str.len() >= 3 {
        let chars: Vec<char> = key_str.chars().collect();
        if chars.len() == 3 && chars[1] == '-' {
            let modifier = chars[0];
            let key_char = chars[2];

            match modifier {
                'C' | 'c' => {
                    // Ctrl+key combination
                    return Ok(tuinix::KeyInput {
                        ctrl: true,
                        alt: false,
                        code: tuinix::KeyCode::Char(key_char),
                    });
                }
                'A' | 'a' | 'M' | 'm' => {
                    // Alt+key combination (Alt or Meta)
                    return Ok(tuinix::KeyInput {
                        ctrl: false,
                        alt: true,
                        code: tuinix::KeyCode::Char(key_char),
                    });
                }
                _ => {}
            }
        }
    }

    // Handle single character input
    if key_str.len() == 1 {
        let ch = key_str.chars().next().unwrap();
        return Ok(tuinix::KeyInput {
            ctrl: false,
            alt: false,
            code: tuinix::KeyCode::Char(ch),
        });
    }

    // If we can't parse it, return an error
    Err(value.invalid(format!("invalid key input format: {:?}", key_str)))
}
