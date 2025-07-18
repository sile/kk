use std::path::PathBuf;

use orfail::OrFail;
use tuinix::{KeyCode, Terminal, TerminalEvent, TerminalInput};

use crate::TerminalFrame;

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    path: PathBuf,
    dirty: bool,
}

impl App {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            path,
            dirty: true,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        loop {
            if self.dirty {
                self.render().or_fail()?;
            }

            match self.terminal.poll_event(None).or_fail()? {
                Some(TerminalEvent::Input(input)) => {
                    let TerminalInput::Key(key_input) = input;

                    if let KeyCode::Char('q') = key_input.code {
                        break;
                    }

                    self.dirty = true;
                }
                Some(TerminalEvent::Resize(_size)) => {
                    self.dirty = true;
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
        writeln!(frame, "File: {}", self.path.display()).or_fail()?;
        writeln!(frame, "\nPress 'q' to quit, any other key to see input").or_fail()?;

        self.terminal.draw(frame).or_fail()?;

        self.dirty = false;

        Ok(())
    }
}
