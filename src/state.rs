use std::path::PathBuf;

use orfail::OrFail;

use crate::buffer::TextBuffer;

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub buffer: TextBuffer,
}

impl State {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let mut buffer = TextBuffer::default();
        buffer.load_file(&path).or_fail()?;
        Ok(Self { path, buffer })
    }
}
