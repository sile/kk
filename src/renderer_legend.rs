use std::fmt::Write;

use orfail::OrFail;
use tuinix::{TerminalRegion, TerminalSize};

use crate::{TerminalFrame, state::State};

#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    pub fn render(&self, _state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let style = tuinix::TerminalStyle::new().dim();
        let reset = tuinix::TerminalStyle::RESET;

        write!(frame, "{style}^C: Quit | ^S: Save | ^O: Open{reset}").or_fail()?;

        Ok(())
    }

    pub fn region(&self, _state: &State, size: TerminalSize) -> TerminalRegion {
        size.to_region().take_top(1).take_right(40)
    }
}
