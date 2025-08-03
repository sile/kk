use std::{io::Write, num::NonZeroUsize, path::PathBuf};

use orfail::OrFail;

#[derive(Debug)]
pub struct CursorAnchorLog {
    log_file_path: PathBuf,
}

impl CursorAnchorLog {
    pub fn append(&self, anchor: CursorAnchor) -> orfail::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.log_file_path)
            .or_fail()?;
        writeln!(file, "{}", anchor).or_fail()?;
        Ok(())
    }

    pub fn recent_anchors(&self) -> orfail::Result<impl Iterator<Item = CursorAnchor>> {
        // TODO: optimize
        let log = std::fs::read_to_string(&self.log_file_path).or_fail()?;
        Ok(log
            .lines()
            .rev()
            .filter_map(|line| line.trim().parse::<CursorAnchor>().ok())
            .collect::<Vec<_>>()
            .into_iter())
    }

    pub fn prev_anchor(&self, current: &CursorAnchor) -> orfail::Result<Option<CursorAnchor>> {
        let n = 1000; // TODO
        if let Some(a) = self
            .recent_anchors()
            .or_fail()?
            .take(n)
            .skip_while(|a| a != current)
            .next()
        {
            return Ok(Some(a));
        }

        Ok(self
            .recent_anchors()
            .or_fail()?
            .take(n)
            .skip_while(|a| a.path != current.path)
            .next())
    }
}

impl Default for CursorAnchorLog {
    fn default() -> Self {
        let dir = std::env::var_os("HOME") // TODO
            .map(PathBuf::from)
            .unwrap_or_default();
        Self {
            log_file_path: dir.join(".kk.anchors"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorAnchor {
    pub path: PathBuf,
    pub line: NonZeroUsize,
    pub char: NonZeroUsize,
}

impl std::fmt::Display for CursorAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.path.display(), self.line, self.char)
    }
}

impl std::str::FromStr for CursorAnchor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, ':');

        let Some(((path_str, line_str), char_str)) =
            parts.next().zip(parts.next()).zip(parts.next())
        else {
            return Err(format!("expected \"PATH:LINE:CHAR\", but got {s:?}"));
        };

        let path = PathBuf::from(path_str);
        let line: NonZeroUsize = line_str
            .parse()
            .map_err(|e| format!("envalid line number: {e}"))?;
        let char: NonZeroUsize = char_str
            .parse()
            .map_err(|e| format!("invalid character position: {e}"))?;

        Ok(Self { path, line, char })
    }
}
