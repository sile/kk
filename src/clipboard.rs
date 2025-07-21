use std::path::PathBuf;

#[derive(Debug)]
pub struct Clipboard {
    path: PathBuf,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            path: PathBuf::from("~/.kak.clipbpard"),
        }
    }
}
