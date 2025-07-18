use std::path::PathBuf;

use orfail::OrFail;
use tuinix::{KeyCode, Terminal, TerminalEvent, TerminalInput};

use crate::TerminalFrame;

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    path: PathBuf,
}

impl App {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self { terminal, path })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        use std::fmt::Write;
        use std::time::Duration;

        // Create initial frame
        let mut frame: TerminalFrame = TerminalFrame::new(self.terminal.size());
        writeln!(frame, "Kak Editor").or_fail()?;
        writeln!(frame, "File: {}", self.path.display()).or_fail()?;
        writeln!(frame, "\nPress 'q' to quit, any other key to see input").or_fail()?;

        // Draw initial frame
        self.terminal.draw(frame).or_fail()?;

        // Event loop
        loop {
            // Poll for events with a timeout
            match self
                .terminal
                .poll_event(Some(Duration::from_millis(100)))
                .or_fail()?
            {
                Some(TerminalEvent::Input(input)) => {
                    let TerminalInput::Key(key_input) = input;

                    // Handle quit command
                    if let KeyCode::Char('q') = key_input.code {
                        break;
                    }

                    // Display the input for debugging
                    let mut frame: TerminalFrame = TerminalFrame::new(self.terminal.size());
                    writeln!(frame, "Kak Editor").or_fail()?;
                    writeln!(frame, "File: {}", self.path.display()).or_fail()?;
                    writeln!(frame, "\nKey pressed: {:?}", key_input).or_fail()?;
                    writeln!(frame, "\nPress 'q' to quit, any other key to see input").or_fail()?;

                    self.terminal.draw(frame).or_fail()?;
                }
                Some(TerminalEvent::Resize(size)) => {
                    // Handle terminal resize
                    let mut frame: TerminalFrame = TerminalFrame::new(size);
                    writeln!(frame, "Kak Editor").or_fail()?;
                    writeln!(frame, "File: {}", self.path.display()).or_fail()?;
                    writeln!(frame, "\nTerminal resized to {}x{}", size.cols, size.rows)
                        .or_fail()?;
                    writeln!(frame, "\nPress 'q' to quit, any other key to see input").or_fail()?;

                    self.terminal.draw(frame).or_fail()?;
                }
                None => {
                    // Timeout elapsed, continue loop
                }
            }
        }

        Ok(())
    }
}
