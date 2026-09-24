/// In-memory clipboard.
#[derive(Debug, Default)]
pub struct Clipboard {
    content: String,
    pub summary_line: String,
}

impl Clipboard {
    pub fn read(&self) -> String {
        self.content.clone()
    }

    pub fn write(&mut self, content: &str) {
        self.summary_line = content.lines().next().unwrap_or_default().to_owned();
        self.content = content.to_owned();
    }
}
