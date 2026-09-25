//! An end-to-end harness that drives the real `kk` binary behind a PTY.
//!
//! The binary is launched as a child of a [`termnix::Session`], which owns the
//! PTY and an I/O-free terminal emulator. Keys and pastes are written through
//! the session's input queue, resizes through [`termnix::Session::resize`], and
//! the rendered grid is read back from the emulator. Nothing in this module
//! touches `kk`'s own code: the tests exercise the same bytes a real terminal
//! would send and assert on the same grid a real terminal would show.

#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

/// How long a `wait_until` predicate may stay unsatisfied before the test
/// fails. Generous because a fresh binary may need to be paged in, and the
/// `kk` loop wakes on a poll rather than a busy spin.
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Upper bound on `pump_io` calls inside one `wait_until` spin.
///
/// The `kk` poll loop can report work that produces no readiness edge, so the
/// spin drains it before blocking on `poll` again.
const PUMP_DRAIN_BUDGET: usize = 64;

/// A scratch file inside the target directory, unique per test name.
///
/// Using the target directory keeps the working tree clean and avoids `/tmp`,
/// which is not reliably writable in every environment these tests run in.
pub fn scratch_file(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("e2e");
    std::fs::create_dir_all(&dir).expect("create scratch directory");
    let path = dir.join(name);
    // Remove any leftover from a previous run before a test uses the file.
    let _ = std::fs::remove_file(&path);
    path
}

/// Drives one `kk` child process behind a PTY.
///
/// The harness owns the [`termnix::Session`]; dropping it closes the session
/// and reaps the child. Tests that need the file contents on disk should read
/// them before the harness is dropped, or call `quit` first.
pub struct KkHarness {
    session: termnix::Session,
    rows: u16,
    cols: u16,
}

impl KkHarness {
    /// Launches the `kk` binary over `path` at an 80x24 terminal.
    ///
    /// The path is passed to `kk` as the positional FILE argument. The caller
    /// is responsible for the file existing; use [`KkHarness::create_new`] to
    /// exercise `--create-new` instead.
    pub fn open(path: &Path) -> Self {
        Self::spawn(path, false)
    }

    /// Launches `kk --create-new` over `path` at an 80x24 terminal.
    ///
    /// The file must not already exist; that is the contract `kk` enforces.
    pub fn create_new(path: &Path) -> Self {
        Self::spawn(path, true)
    }

    /// Launches `kk` over `path`, optionally with `--create-new`.
    fn spawn(path: &Path, create_new: bool) -> Self {
        let mut command = Command::new(kk_binary());
        if create_new {
            command.arg("--create-new");
        }
        command.arg(path);

        let rows = 24;
        let cols = 80;
        let session = termnix::Session::new(&mut command, size(rows, cols))
            .expect("failed to start the kk binary behind a PTY");
        Self {
            session,
            rows,
            cols,
        }
    }

    /// Sends a plain character key with no modifiers.
    pub fn send_char(&mut self, ch: char) {
        self.send_key(termnix::KeyCode::Char(ch), termnix::Modifiers::new());
    }

    /// Sends every character of `text`, one key event at a time.
    pub fn send_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.send_char(ch);
        }
    }

    /// Sends a key with modifiers.
    ///
    /// The session encodes the event using the terminal modes `kk` has put it
    /// in, so this is how a real terminal's bytes are reproduced.
    pub fn send_key(&mut self, code: termnix::KeyCode, modifiers: termnix::Modifiers) {
        let event = termnix::KeyEvent { code, modifiers };
        self.session
            .enqueue_input(termnix::Input::Key(event))
            .expect("failed to enqueue a key event");
    }

    /// Sends a printable character with Ctrl held.
    pub fn send_ctrl(&mut self, ch: char) {
        self.send_key(termnix::KeyCode::Char(ch), termnix::Modifiers::new().ctrl());
    }

    /// Sends a bare Escape key.
    ///
    /// `kk` holds a lone `ESC` byte for a short timeout before committing it,
    /// so callers should `wait_until` on the effect rather than assume the key
    /// is processed the instant it is enqueued.
    pub fn send_escape(&mut self) {
        self.send_key(termnix::KeyCode::Escape, termnix::Modifiers::new());
    }

    /// Resizes the terminal, delivering SIGWINCH to `kk`.
    ///
    /// The emulator's size is updated alongside the kernel PTY size, so the
    /// visible row and column counts follow the new grid.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.session
            .resize(size(rows, cols))
            .expect("failed to resize the PTY");
        self.rows = rows;
        self.cols = cols;
    }

    /// Reads the visible grid back as one trimmed string per row.
    ///
    /// Scrollback is included ahead of the visible screen, matching how
    /// `termnix`'s own headless example builds its snapshot.
    pub fn screen_rows(&self) -> Vec<String> {
        let state = self.session.terminal_state();
        let mut rows = Vec::new();
        for line in state.scrollback_lines() {
            rows.push(line_text(line.cells()));
        }
        for row in 0..self.rows {
            let mut cells = Vec::with_capacity(self.cols as usize);
            for col in 0..self.cols {
                let cell = state
                    .cell(termnix::Position { row, col })
                    .expect("cell within the emulator grid");
                cells.push(cell);
            }
            rows.push(line_text(&cells));
        }
        rows
    }

    /// Returns the first visible row containing `needle`, trimmed.
    pub fn find_row(&self, needle: &str) -> Option<String> {
        self.screen_rows().into_iter().find(|r| r.contains(needle))
    }

    /// Whether any visible row contains `needle`.
    pub fn screen_contains(&self, needle: &str) -> bool {
        self.find_row(needle).is_some()
    }

    /// The whole screen with rows joined by newlines.
    pub fn screen_text(&self) -> String {
        self.screen_rows().join("\n")
    }

    /// Pumps the session until `pred` holds, or panics after `WAIT_TIMEOUT`.
    ///
    /// `pred` receives the harness so it can read the grid; it must not mutate
    /// the harness, which keeps every assertion a pure read.
    pub fn wait_until<F>(&mut self, what: &str, pred: F)
    where
        F: Fn(&KkHarness) -> bool,
    {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            self.pump();
            if pred(self) {
                return;
            }
            if Instant::now() >= deadline {
                panic!(
                    "timed out waiting for: {what}\nscreen:\n{}",
                    self.screen_text()
                );
            }
            self.poll_once(deadline);
        }
    }

    /// Waits until the screen shows `needle`, printing the grid on failure.
    pub fn wait_for_text(&mut self, needle: &str) {
        self.wait_until(needle, |h| h.screen_contains(needle));
    }

    /// Waits for `kk` to exit, after the caller has asked it to quit.
    ///
    /// Returns the exit status once the child is reaped and the PTY drained.
    pub fn wait_for_exit(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            self.pump();
            if let Some(status) = self.session.exit_status()
                && (self.session.status() == termnix::SessionStatus::Eof
                    || (self.session.fd().is_none() && !self.session.needs_pump()))
            {
                return status;
            }
            if let Err(err) = self.session.try_wait() {
                panic!("try_wait failed: {err}");
            }
            if Instant::now() >= deadline {
                panic!(
                    "timed out waiting for kk to exit\nscreen:\n{}",
                    self.screen_text()
                );
            }
            self.poll_once(deadline);
        }
    }

    /// Sends `C-c` and waits for the child to exit.
    pub fn quit(&mut self) -> std::process::ExitStatus {
        self.send_ctrl('c');
        self.wait_for_exit()
    }

    /// Advances the session by one bounded pump plus a drain of pending work.
    fn pump(&mut self) {
        self.session
            .pump_io(termnix::PumpBudget::default())
            .expect("pump_io failed");
        let mut budget = PUMP_DRAIN_BUDGET;
        while self.session.needs_pump() && budget > 0 {
            self.session
                .pump_io(termnix::PumpBudget::default())
                .expect("pump_io failed");
            budget -= 1;
        }
    }

    /// Blocks until the child has something to say, or the deadline passes.
    fn poll_once(&mut self, deadline: Instant) {
        if self.session.needs_pump() {
            return;
        }
        let Some(fd) = self.session.fd() else {
            return;
        };
        let interests = self.session.interests();
        let mut events = 0;
        if interests.readable {
            events |= libc::POLLIN;
        }
        if interests.writable {
            events |= libc::POLLOUT;
        }
        if events == 0 {
            return;
        }
        let timeout = poll_timeout_ms(deadline);
        let mut pollfd = libc::pollfd {
            fd: fd as libc::c_int,
            events,
            revents: 0,
        };
        // SAFETY: `poll` receives one fully initialized `pollfd` and a count of
        // one, and only writes to `revents` within that array.
        let rc = unsafe { libc::poll(&mut pollfd, 1, timeout) };
        if rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                panic!("poll failed: {err}");
            }
        }
    }
}

impl Drop for KkHarness {
    fn drop(&mut self) {
        self.session.close();
    }
}

/// Renders one row of cells as a string, dropping wide-character padding.
fn line_text(cells: &[termnix::Cell]) -> String {
    let mut line = String::new();
    for cell in cells {
        if cell.width == 0 {
            continue;
        }
        line.push(cell.ch);
    }
    line.trim_end().to_string()
}

/// A non-zero [`termnix::Size`] from plain `u16` dimensions.
fn size(rows: u16, cols: u16) -> termnix::Size {
    termnix::Size {
        rows: std::num::NonZeroU16::new(rows).expect("rows must be non-zero"),
        cols: std::num::NonZeroU16::new(cols).expect("cols must be non-zero"),
    }
}

/// Milliseconds until `deadline`, clamped to a finite non-negative value.
fn poll_timeout_ms(deadline: Instant) -> libc::c_int {
    let now = Instant::now();
    if now >= deadline {
        return 0;
    }
    let ms = deadline.duration_since(now).as_millis();
    libc::c_int::try_from(ms).unwrap_or(libc::c_int::MAX).max(0)
}

/// The path to the `kk` binary cargo built for this test run.
fn kk_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_kk"))
}
