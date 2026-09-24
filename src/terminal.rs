//! Unicode-aware terminal utilities for character width calculation and rendering.

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

/// A terminal frame that uses Unicode-aware character width estimation.
pub type UnicodeTerminalFrame = tuinix::TerminalFrame<UnicodeCharWidthEstimator>;

/// A character width estimator that uses Unicode width calculation.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnicodeCharWidthEstimator;

impl tuinix::EstimateCharWidth for UnicodeCharWidthEstimator {
    fn estimate_char_width(&self, c: char) -> usize {
        char_cols(c)
    }
}
