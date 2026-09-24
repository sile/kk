//! Unicode-aware terminal utilities for character width calculation and rendering.

use tuinix::{Char, Frame, Position, Style};

/// Calculates the display width of a string in terminal columns.
pub fn str_cols(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// Calculates the display width of a character in terminal columns.
///
/// Returns 0 for characters that have no width (like control characters or
/// zero-width combining characters).
pub fn char_cols(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Paints `text` into `frame` starting at `at`, using `style` for every
/// character, and returns the position just past the last character.
///
/// Newlines move to the next row, tabs advance to the next tab stop, and other
/// control characters are skipped. Characters whose width is zero are ignored.
pub fn put_str(frame: &mut Frame, at: Position, text: &str, style: Style) -> Position {
    let mut at = at;
    for c in text.chars() {
        match c {
            '\n' => at = at.next_line(),
            '\t' => {
                if let Some(tab) = std::num::NonZeroUsize::new(8) {
                    at = at.next_tab_stop(tab);
                }
            }
            c => {
                let width = char_cols(c);
                let Some(ch) = Char::new(c, width, style) else {
                    continue;
                };
                at = frame.put_char(at, ch);
            }
        }
    }
    at
}
