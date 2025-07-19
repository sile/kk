use std::fmt::Write;

use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalRegion, TerminalSize};

use crate::{TerminalFrame, state::State};

#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        self.render_to_writer(state, frame).or_fail()?;
        Ok(())
    }

    pub fn region(&self, state: &State, size: TerminalSize) -> TerminalRegion {
        let mut detector = SizeDetector::default();
        self.render_to_writer(state, &mut detector).expect("bug");
        if !size.contains(detector.size.to_region().bottom_right()) {
            TerminalRegion::default()
        } else {
            size.to_region()
                .take_top(detector.size.rows)
                .take_right(detector.size.cols)
        }
    }

    fn render_to_writer<W: Write>(&self, _state: &State, mut writer: W) -> orfail::Result<()> {
        writeln!(writer, "^C: Quit | ^S: Save | ^O: Open ").or_fail()?;

        Ok(())
    }
}

#[derive(Debug, Default)]
struct SizeDetector {
    cursor: TerminalPosition,
    size: TerminalSize,
}

impl Write for SizeDetector {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for ch in s.chars() {
            match ch {
                '\n' => {
                    self.size.rows += 1;
                    self.size.cols = self.size.cols.max(self.cursor.col);
                    self.cursor.col = 0;
                    self.cursor.row += 1;
                }
                _ => {
                    self.cursor.col += 1;
                }
            }
        }
        self.size.cols = self.size.cols.max(self.cursor.col);

        Ok(())
    }
}
