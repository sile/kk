use std::{fmt::Write, io::Write as _, path::PathBuf};

use mame::UnicodeTerminalFrame as TerminalFrame;
use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalRegion};

use crate::{
    action::GrepAction,
    buffer::{TextBuffer, TextPosition},
    state::State,
};

#[derive(Debug)]
pub struct GrepMode {
    pub action: GrepAction,
    pub query: Vec<char>,
    pub cursor: usize,
    query_history_index: Option<usize>,
}

impl GrepMode {
    pub fn new(action: GrepAction) -> Self {
        Self {
            action,
            query: Vec::new(),
            cursor: 0,
            query_history_index: None,
        }
    }

    pub fn cursor_position(&self, region: TerminalRegion) -> TerminalPosition {
        let mut frame = TerminalFrame::new(region.size);
        let mut pos = region.position;

        // TODO: factor out
        let _ = write!(frame, "$ {} ", self.action.command);
        for arg in &self.action.args {
            let _ = write!(frame, "{arg} ");
        }
        for ch in self.query.iter().take(self.cursor) {
            let _ = write!(frame, "{ch}");
        }
        pos.col = frame.cursor().col;

        pos
    }

    pub fn handle_char_insert(&mut self, key: tuinix::KeyInput) {
        let tuinix::KeyCode::Char(ch) = key.code else {
            return;
        };
        self.query.insert(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn grep(&mut self, buffer: &TextBuffer) -> orfail::Result<Highlight> {
        if self.query.is_empty() {
            return Ok(Highlight::default());
        }

        let buffer = buffer.to_single_text();
        let output = self.execute_command(&buffer).or_fail()?;
        let dir = std::env::var_os("HOME") // TODO
            .map(PathBuf::from)
            .unwrap_or_default();
        std::fs::write(dir.join(".kk.highlight"), &output).or_fail()?;

        Highlight::parse(&output, &buffer).or_fail()
    }

    pub fn next_query(&mut self) -> orfail::Result<Option<String>> {
        // TODO: optimize
        let Some(i) = self.query_history_index.and_then(|i| i.checked_sub(1)) else {
            self.query_history_index = None;
            return Ok(None);
        };

        let path = self.query_history_path();
        let text = std::fs::read_to_string(&path).or_fail()?;
        let Some(query) = text.lines().nth_back(i) else {
            self.query_history_index = None;
            return Ok(None);
        };
        self.query_history_index = Some(i);
        Ok(Some(query.to_owned()))
    }

    pub fn prev_query(&mut self) -> orfail::Result<Option<String>> {
        // TODO: optimize
        let i = self.query_history_index.map(|i| i + 1).unwrap_or(0);

        let path = self.query_history_path();
        let text = std::fs::read_to_string(&path).or_fail()?;
        let Some(query) = text.lines().nth_back(i) else {
            self.query_history_index = None;
            return Ok(None);
        };
        self.query_history_index = Some(i);
        Ok(Some(query.to_owned()))
    }

    pub fn save_query(&self) -> orfail::Result<()> {
        let path = self.query_history_path();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .or_fail()?;
        writeln!(file, "{}", self.query.iter().collect::<String>()).or_fail()?;
        Ok(())
    }

    fn query_history_path(&self) -> PathBuf {
        let dir = std::env::var_os("HOME") // TODO
            .map(PathBuf::from)
            .unwrap_or_default();
        dir.join(".kk.grep-queries")
    }

    fn execute_command(&self, buffer: &str) -> orfail::Result<String> {
        let mut cmd = std::process::Command::new(&self.action.command);
        for arg in &self.action.args {
            cmd.arg(arg);
        }
        cmd.arg(self.query.iter().copied().collect::<String>());

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .or_fail_with(|e| format!("Failed to execute grep command: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            write!(stdin, "{buffer}").or_fail()?;
            stdin.flush().or_fail()?;
        }

        let output = child
            .wait_with_output()
            .or_fail_with(|e| format!("Failed to wait for command: {e}"))?;

        match output.status.code() {
            Some(0 | 1) => {}
            _ => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(orfail::Failure::new(format!(
                    "Grep command failed: {}",
                    stderr.trim()
                )));
            }
        }
        String::from_utf8(output.stdout).or_fail()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HighlightItem {
    pub start_position: TextPosition,
    pub end_position: TextPosition,
}

#[derive(Debug, Default)]
pub struct Highlight {
    pub items: Vec<HighlightItem>,
}

impl Highlight {
    fn parse(output: &str, input: &str) -> orfail::Result<Self> {
        let mut items = Vec::new();
        for line in output.lines() {
            let (byte_offset, text) = line.trim().split_once(':').or_fail()?;
            let start_byte_offset = byte_offset.parse::<usize>().or_fail()?;
            let end_byte_offset = start_byte_offset + text.len();
            items.push(HighlightItem {
                start_position: byte_offset_to_text_position(input, start_byte_offset).or_fail()?,
                end_position: byte_offset_to_text_position(input, end_byte_offset).or_fail()?,
            });
        }
        items.sort_by_key(|x| x.start_position);
        Ok(Self { items })
    }

    pub fn contains(&self, pos: TextPosition) -> bool {
        self.items
            .iter()
            .any(|item| item.start_position <= pos && pos < item.end_position)
    }
}

fn byte_offset_to_text_position(text: &str, offset: usize) -> orfail::Result<TextPosition> {
    if offset > text.len() {
        return Err(orfail::Failure::new("Byte offset exceeds text length"));
    }

    let mut row = 0;
    let mut col = 0;
    let mut current_offset = 0;

    for ch in text.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            row += 1;
            col = 0;
        } else {
            col += mame::char_cols(ch);
        }

        current_offset += ch.len_utf8();
    }

    Ok(TextPosition { row, col })
}

#[derive(Debug)]
pub struct GrepQueryRenderer;

impl GrepQueryRenderer {
    pub fn render(&self, state: &State, frame: &mut TerminalFrame) -> orfail::Result<()> {
        let Some(grep) = &state.grep_mode else {
            unreachable!();
        };

        write!(frame, "$ {} ", grep.action.command).or_fail()?;
        for arg in &grep.action.args {
            write!(frame, "{arg} ").or_fail()?;
        }
        for ch in &grep.query {
            write!(frame, "{ch}").or_fail()?;
        }
        writeln!(frame).or_fail()?;
        Ok(())
    }
}
