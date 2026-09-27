//! A clipboard that lives in memory for the lifetime of the process.

/// The editor's clipboard.
///
/// It holds text copied or cut from the buffer. Nothing is persisted, so the
/// contents last only as long as the process does.
#[derive(Debug, Default)]
pub struct Clipboard {
    content: String,
}

impl Clipboard {
    /// Returns a copy of the current contents.
    pub fn read(&self) -> String {
        self.content.clone()
    }

    /// Replaces the contents with `content`.
    pub fn write(&mut self, content: &str) {
        self.content = content.to_owned();
    }

    /// Appends `content` to the current contents.
    ///
    /// The clipboard holds one entry, so a run of kills that the editor treats
    /// as one collects here rather than replacing what came before.
    pub fn append(&mut self, content: &str) {
        self.content.push_str(content);
    }

    /// Returns the first line of the contents, for the status line.
    ///
    /// It is empty when the contents are empty.
    pub fn summary_line(&self) -> &str {
        self.content.lines().next().unwrap_or_default()
    }
}
