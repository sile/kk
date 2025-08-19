use std::fmt::Write;

use mame::UnicodeTerminalFrame as TerminalFrame;
use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalRegion, TerminalSize};

use crate::{config::Config, state::State};

#[derive(Debug)]
pub struct LegendRenderer;

impl LegendRenderer {
    pub fn render(
        &self,
        config: &Config,
        state: &State,
        frame: &mut TerminalFrame,
    ) -> orfail::Result<()> {
        self.render_to_writer(config, state, Some(frame.size().cols), frame)
            .or_fail()?;
        Ok(())
    }

    pub fn region(&self, config: &Config, state: &State, size: TerminalSize) -> TerminalRegion {
        let mut detector = SizeDetector::default();
        detector.size.rows += 1; // for bottom border

        self.render_to_writer(config, state, None, &mut detector)
            .expect("bug");
        let legend_size = detector.finish();

        if !size.contains(legend_size.to_region().bottom_right()) {
            TerminalRegion::default()
        } else {
            size.to_region()
                .take_top(legend_size.rows)
                .take_right(legend_size.cols)
        }
    }

    fn render_to_writer<W: Write>(
        &self,
        config: &Config,
        state: &State,
        cols: Option<usize>,
        mut writer: W,
    ) -> orfail::Result<()> {
        for binding in config.keybindings.iter(&state.context).or_fail()? {
            if !binding.visible {
                continue;
            }

            let action = &binding.action;
            if let Some(label) = config.keylabels.get(&binding.key) {
                writeln!(writer, "│ {label}: {action} ").or_fail()?;
            } else {
                let label = binding.key;
                writeln!(writer, "│ {label}: {action} ").or_fail()?;
            }
        }

        if let Some(cols) = cols {
            writeln!(writer, "└{}", "─".repeat(cols.saturating_sub(1))).or_fail()?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
struct SizeDetector {
    cursor: TerminalPosition,
    size: TerminalSize,
}

impl SizeDetector {
    fn finish(mut self) -> TerminalSize {
        if self.size.cols > 0 {
            self.size.rows += 1;
        }
        TerminalSize::rows_cols(self.size.rows, self.size.cols)
    }
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
