use std::{num::NonZeroUsize, path::PathBuf};

#[derive(Debug, Clone)]
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
