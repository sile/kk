//! Unicode-aware terminal utilities for character width calculation and rendering.

/// Calculates the display width of a string in terminal columns.
pub fn str_cols(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// The number of columns a tab advances to, the same stop [`put_str`] moves to.
///
/// A tab is a run of blank columns up to the next multiple of this width, so
/// this is the one place the width is written: the renderer's tab stop and the
/// buffer's column model both read it from here.
pub const TAB_WIDTH: usize = 8;

/// Calculates the display width of a character in terminal columns.
///
/// Returns 0 for characters that have no width (like control characters or
/// zero-width combining characters).
pub fn char_cols(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Calculates the number of columns the character at column `col` occupies.
///
/// A tab occupies the columns from `col` up to the next tab stop, so it
/// depends on where it starts; every other character has the same width
/// wherever it is, which is [`char_cols`].
pub fn char_cols_at(c: char, col: usize) -> usize {
    match c {
        '\t' => TAB_WIDTH - col % TAB_WIDTH,
        c => char_cols(c),
    }
}

/// Paints `text` into `frame` starting at `at`, using `style` for every
/// character, and returns the position just past the last character.
///
/// Newlines move to the next row, tabs advance to the next [`TAB_WIDTH`] stop,
/// and other control characters are skipped. Characters whose width is zero
/// are ignored.
///
/// A tab paints nothing: it moves `at` past the columns it covers, leaving
/// them for whatever was already drawn there. A caller that paints its own
/// columns -- as the text area does, one character at a time -- measures a tab
/// with [`char_cols_at`] instead of relying on this.
pub fn put_str(
    frame: &mut tuinix::Frame,
    at: tuinix::Position,
    text: &str,
    style: tuinix::Style,
) -> tuinix::Position {
    let mut at = at;
    for c in text.chars() {
        match c {
            '\n' => at = at.next_line(),
            '\t' => {
                if let Some(tab) = std::num::NonZeroUsize::new(TAB_WIDTH) {
                    at = at.next_tab_stop(tab);
                }
            }
            c => {
                let width = char_cols(c);
                let Some(ch) = tuinix::Char::new(c, width, style) else {
                    continue;
                };
                at = frame.put_char(at, ch);
            }
        }
    }
    at
}
