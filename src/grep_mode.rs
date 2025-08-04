use std::fmt::Write;

use orfail::OrFail;
use tuinix::{TerminalPosition, TerminalRegion};

use crate::{action::GrepAction, mame::TerminalFrame, state::State};

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

// pub fn handle_grep(&mut self, action: &GrepAction) -> orfail::Result<()> {
//     // TODO: factor out with handle_external_command()
//     self.finish_editing();

//     let mut cmd = std::process::Command::new(&action.command);

//     for arg in &action.args {
//         cmd.arg(arg);
//     }

//     let stdin_input = if let Some(mark_pos) = self.mark {
//         let cursor_pos = self.cursor_position();
//         let (start, end) = if mark_pos <= cursor_pos {
//             (mark_pos, cursor_pos)
//         } else {
//             (cursor_pos, mark_pos)
//         };
//         self.get_text_in_range(start, end)
//     } else {
//         None
//     };
//     cmd.stdin(std::process::Stdio::piped());
//     cmd.stdout(std::process::Stdio::piped());
//     cmd.stderr(std::process::Stdio::piped());

//     let mut child = match cmd.spawn() {
//         Err(e) => {
//             self.set_message(format!("Failed to execute command: {}", e));
//             return Ok(());
//         }
//         Ok(child) => child,
//     };

//     if let Some(mut stdin) = child.stdin.take() {
//         use std::io::Write;

//         if let Some(text) = stdin_input {
//             let _ = stdin.write_all(text.as_bytes());
//         } else {
//             let mut writer = BufWriter::new(stdin);
//             for line in &self.buffer.text {
//                 for ch in &line.0 {
//                     write!(writer, "{ch}").or_fail()?;
//                 }
//                 writeln!(writer).or_fail()?;
//             }
//             writer.flush().or_fail()?;
//         }
//     }

//     let output = match child.wait_with_output() {
//         Err(e) => {
//             self.set_message(format!("Failed to wait for command: {}", e));
//             return Ok(());
//         }
//         Ok(output) => output,
//     };

//     if !output.status.success() {
//         let stderr = String::from_utf8_lossy(&output.stderr);
//         self.set_message(format!("Grep command failed: {}", stderr.trim()));
//         return Ok(());
//     }

//     let dir = std::env::var_os("HOME") // TODO
//         .map(PathBuf::from)
//         .unwrap_or_default();
//     std::fs::write(dir.join(".kk.highlight"), &output.stdout).or_fail()?;
//     Ok(())
// }
