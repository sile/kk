use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    path: PathBuf,
}

impl App {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn run(self) -> orfail::Result<()> {
        Ok(())
    }
}
