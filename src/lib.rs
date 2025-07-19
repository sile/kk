pub mod app;
pub mod buffer;
pub mod renderer_legend;
pub mod renderer_message_line;
pub mod renderer_status_line;
pub mod renderer_text_area;
pub mod state;

pub type TerminalFrame = tuinix::TerminalFrame<UnicodeCharWidthEstimator>;

#[derive(Debug, Default)]
pub struct UnicodeCharWidthEstimator;

impl tuinix::EstimateCharWidth for UnicodeCharWidthEstimator {
    fn estimate_char_width(&self, c: char) -> usize {
        unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
    }
}
