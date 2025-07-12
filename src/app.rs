use std::path::PathBuf;

use orfail::OrFail;
use tuinix::Terminal;

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

    pub fn run(self) -> orfail::Result<()> {
        Ok(())
    }
}
