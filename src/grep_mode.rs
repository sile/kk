use std::fmt::Write;

use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalRegion};

use crate::{action::GrepAction, buffer::TextBuffer, mame::TerminalFrame, state::State};

#[derive(Debug)]
pub struct GrepMode {
    pub action: GrepAction,
    pub query: Vec<char>,
    pub cursor: usize,
}

impl GrepMode {
    pub fn new(action: GrepAction) -> Self {
        Self {
            action,
            query: Vec::new(),
            cursor: 0,
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

    pub fn grep(&mut self, buffer: &TextBuffer) -> orfail::Result<()> {
        self.execute_command(buffer).or_fail()?;
        // let dir = std::env::var_os("HOME") // TODO
        //     .map(PathBuf::from)
        //     .unwrap_or_default();
        // std::fs::write(dir.join(".kk.highlight"), &output.stdout).or_fail()?;

        Ok(())
    }

    fn execute_command(&self, buffer: &TextBuffer) -> orfail::Result<String> {
        let mut cmd = std::process::Command::new(&self.action.command);
        for arg in &self.action.args {
            cmd.arg(arg);
        }
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .or_fail_with(|e| format!("Failed to execute grep command: {e}"))?;

        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;

            let mut writer = std::io::BufWriter::new(stdin);
            for line in &buffer.text {
                for ch in &line.0 {
                    write!(writer, "{ch}").or_fail()?;
                }
                writeln!(writer).or_fail()?;
            }
            writer.flush().or_fail()?;
        }

        let output = child
            .wait_with_output()
            .or_fail_with(|e| format!("Failed to wait for command: {e}"))?;

        output.status.success().or_fail_with(|()| {
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("Grep command failed: {}", stderr.trim())
        })?;
        String::from_utf8(output.stdout).or_fail()
    }
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
