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
        self.summary_line = content.lines().next().unwrap_or_default().to_owned();
        self.content = content.to_owned();
    }
}
