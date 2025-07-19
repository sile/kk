use std::path::PathBuf;

use orfail::OrFail;
use tuinix::{KeyCode, Terminal, TerminalEvent, TerminalInput, TerminalRegion};

use crate::{
    TerminalFrame, renderer_message_line::MessageLineRenderer,
    renderer_status_line::StatusLineRenderer, state::State,
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    state: State,
    message_line: MessageLineRenderer,
    status_line: StatusLineRenderer,
}

impl App {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            state: State::new(path).or_fail()?,
            message_line: MessageLineRenderer,
            status_line: StatusLineRenderer,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        let mut dirty = true;
        self.state.set_message("Started");

        loop {
            if dirty {
                self.render().or_fail()?;
                dirty = false;
            }

            match self.terminal.poll_event(None).or_fail()? {
                Some(TerminalEvent::Input(input)) => {
                    let TerminalInput::Key(key_input) = input;

                    if let KeyCode::Char('q') = key_input.code {
                        break;
                    }

                    dirty = true;
                }
                Some(TerminalEvent::Resize(_size)) => {
                    dirty = true;
                }
                None => {}
            }
        }

        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        use std::fmt::Write;

        let mut frame = TerminalFrame::new(self.terminal.size());

        writeln!(frame, "Kak Editor").or_fail()?;
        writeln!(frame, "File: {}", self.state.path.display()).or_fail()?;
        writeln!(frame, "\nPress 'q' to quit, any other key to see input").or_fail()?;

        // Add status line rendering
        let status_line_region = frame.size().to_region().take_bottom(2).take_top(1);
        self.render_region(&mut frame, status_line_region, |state, frame| {
            self.status_line.render(state, frame).or_fail()
        })?;

        let message_line_region = frame.size().to_region().take_bottom(1);
        self.render_region(&mut frame, message_line_region, |state, frame| {
            self.message_line.render(state, frame).or_fail()
        })?;

        self.terminal.draw(frame).or_fail()?;

        self.state.message = None;
        Ok(())
    }

    fn render_region<F>(
        &self,
        frame: &mut TerminalFrame,
        region: TerminalRegion,
        f: F,
    ) -> orfail::Result<()>
    where
        F: FnOnce(&State, &mut TerminalFrame) -> orfail::Result<()>,
    {
        let mut sub_frame = TerminalFrame::new(region.size);
        f(&self.state, &mut sub_frame).or_fail()?;
        frame.draw(region.position, &sub_frame);
        Ok(())
    }
}
