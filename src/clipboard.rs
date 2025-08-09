use std::path::PathBuf;

use orfail::OrFail;

#[derive(Debug)]
pub struct Clipboard {
    path: PathBuf,
    pub summary_line: String,
}

impl Clipboard {
    pub fn read(&self) -> orfail::Result<String> {
        if self.path.exists() {
            std::fs::read_to_string(&self.path).or_fail()
        } else {
            Ok("".to_owned())
        }
    }

    pub fn write(&mut self, content: &str) -> orfail::Result<()> {
        // TODO: Update when the file is modified
        self.summary_line = content.lines().next().unwrap_or_default().to_owned();

        // TODO: File::lock()
        std::fs::write(&self.path, content).or_fail()
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        let dir = std::env::var_os("HOME") // TODO
            .map(PathBuf::from)
            .unwrap_or_default();
        Self {
            path: dir.join(".kk.clipboard"),
            summary_line: String::new(),
        }
    }
}
