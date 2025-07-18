use std::path::PathBuf;

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
}

impl State {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        Ok(Self { path })
    }
}
