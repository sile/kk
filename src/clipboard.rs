//! A clipboard that lives in memory for the lifetime of the process.

/// The editor's clipboard.
///
/// It holds text copied or cut from the buffer. Nothing is persisted, so the
/// contents last only as long as the process does.
#[derive(Debug, Default)]
pub struct Clipboard {
    content: String,

    /// The first line of the current contents, for the status line.
    pub summary_line: String,
}

impl Clipboard {
    /// Returns a copy of the current contents.
    pub fn read(&self) -> String {
        self.content.clone()
    }

    /// Replaces the contents with `content`.
    ///
    /// [`summary_line`](Self::summary_line) is updated to `content`'s first
    /// line, or the empty string when `content` is empty.
    pub fn write(&mut self, content: &str) {
        self.content = content.to_owned();
        self.update_summary();
    }

    /// Appends `content` to the current contents.
    ///
    /// The clipboard holds one entry, so a run of kills that the editor treats
    /// as one collects here rather than replacing what came before. The summary
    /// is recomputed from the joined text, so it stays on whatever the first
    /// line of the run is.
    pub fn append(&mut self, content: &str) {
        self.content.push_str(content);
        self.update_summary();
    }

    fn update_summary(&mut self) {
        self.summary_line = self.content.lines().next().unwrap_or_default().to_owned();
    }
}
