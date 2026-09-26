//! The I/O edge: poll loop, raw terminal, file access, and frame diffing.
//!
//! This module is compiled into the binary only (`mod app;` in `main.rs`); the
//! library stays Sans I/O. It owns the [`tuinix::TerminalDriver`], and it is the
//! one place that reads the edited file, writes it back on save, and reads
//! input from and paints frames to the terminal. The [`kk`] core it drives takes
//! plain values and returns plain values, and never calls back into here.
//!
//! The one timeout is for the lone `ESC` byte: `tuinix`'s decoder holds it back
//! because it cannot tell Escape from the start of a sequence, so the loop waits
//! [`ESCAPE_TIMEOUT_MS`] and then commits it.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// How long to wait for the rest of an escape sequence before a lone `ESC` byte
/// is treated as the Escape key.
const ESCAPE_TIMEOUT_MS: libc::c_int = 50;

/// Owns the terminal edge and drives the core until the user quits.
#[derive(Debug)]
pub struct App {
    driver: tuinix::TerminalDriver,
    input: tuinix::InputDecoder,
    prev_frame: Option<tuinix::Frame>,
    path: PathBuf,
    context: kk::Context,
    state: kk::State,
    text_area: kk::TextAreaRenderer,
    message_line: kk::MessageLineRenderer,
    status_line: kk::StatusLineRenderer,
    legend: kk::LegendRenderer,
    legend_visible: bool,
    exit: bool,
}

impl App {
    /// Reads `path` from disk and takes over the terminal.
    ///
    /// Reading the file here keeps the core free of the file system: the core is
    /// handed an already-loaded [`TextBuffer`](kk::TextBuffer), and this edge
    /// keeps `path` for later saves and reloads.
    ///
    /// A missing file is created by `--create-new`, as
    /// [`std::fs::File::create_new`] would; if the file already exists that is an
    /// error, exactly as with `create_new(true)`. Other read failures are still
    /// errors too. The first message says whether the file was opened or created.
    pub fn new<P: AsRef<Path>>(path: P, create_new: bool) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        // `create_new` leaves the buffer empty without a read: the call only
        // succeeds when it has just brought an empty file into existence.
        let text = if create_new {
            std::fs::File::create_new(&path)?;
            String::new()
        } else {
            std::fs::read_to_string(&path)?
        };

        let buffer = kk::TextBuffer::from_text(&text);

        let mut state = kk::State::new(buffer);
        state.set_message(if create_new { "Created" } else { "Opened" });
        let driver = tuinix::TerminalDriver::new()?;

        Ok(Self {
            driver,
            input: tuinix::InputDecoder::new(),
            prev_frame: None,
            path,
            state,
            context: kk::Context::Edit,
            text_area: kk::TextAreaRenderer,
            message_line: kk::MessageLineRenderer,
            status_line: kk::StatusLineRenderer,
            legend: kk::LegendRenderer,
            legend_visible: true,
            exit: false,
        })
    }

    /// Runs the poll loop until the user quits.
    pub fn run(mut self) -> std::io::Result<()> {
        let mut fds = [
            libc::pollfd {
                fd: self.driver.resize_signal_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.driver.input_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        while !self.exit {
            self.render()?;

            // A lone `ESC` byte is ambiguous, so wait only briefly while one is
            // held so it is reported as Escape promptly.
            let timeout = if self.input.has_uncommitted_escape() {
                ESCAPE_TIMEOUT_MS
            } else {
                -1
            };
            #[expect(
                unsafe_code,
                reason = "libc::poll is the only way to wait on the resize and input fds at once"
            )]
            let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                // `poll` is never restarted by `SA_RESTART`, so SIGWINCH makes it
                // return `EINTR`; the resize byte is already in the pipe, so retry.
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }

            if fds
                .iter()
                .any(|fd| fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0)
            {
                return Err(std::io::Error::other("terminal closed"));
            }

            if n == 0 {
                self.input.commit_escape();
            }

            if fds[0].revents & libc::POLLIN != 0 {
                self.driver.handle_resize_signal()?;
            }

            if fds[1].revents & libc::POLLIN != 0 {
                self.read_input()?;
            }

            while let Some(input) = self.input.next() {
                self.handle_input(input)?;
            }
        }

        Ok(())
    }

    fn read_input(&mut self) -> std::io::Result<()> {
        let mut raw = [0u8; 256];
        loop {
            match self.driver.read(&mut raw) {
                Ok(n @ 1..) => self.input.feed(&raw[..n]),
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn handle_input(&mut self, input: tuinix::Input) -> std::io::Result<()> {
        let Some(resolved) = kk::resolve(self.context, &input) else {
            self.state
                .set_message(format!("No action found: '{}'", kk::input(&input)));
            return Ok(());
        };

        if let Some(action) = resolved.action {
            self.handle_action(action, &input)?;
        }

        if let Some(context) = resolved.context {
            self.context = context;
        }

        Ok(())
    }

    /// Carries out a core [`Action`](kk::Action) at the edge.
    ///
    /// It returns [`std::io::Result`] because a few actions touch the file
    /// system (save and reload); every other arm is infallible and simply does
    /// not use `?`.
    fn handle_action(&mut self, action: kk::Action, input: &tuinix::Input) -> std::io::Result<()> {
        match action {
            kk::Action::Quit => {
                self.exit = true;
            }
            kk::Action::Cancel => {
                self.state.mark = None;
                self.state.grep_mode = None;
                self.state.highlight = kk::Highlight::default();
                self.state.set_message("Canceled");
            }
            kk::Action::BufferSave => {
                self.handle_buffer_save()?;
                self.state.mark = None;
                self.state.grep_mode = None;
                self.state.highlight = kk::Highlight::default();
            }
            kk::Action::BufferReload => self.handle_buffer_reload()?,
            kk::Action::BufferUndo => self.state.handle_buffer_undo(),
            kk::Action::LegendToggle => self.legend_visible = !self.legend_visible,
            kk::Action::CursorUp => self.state.handle_cursor_up(),
            kk::Action::CursorDown => self.state.handle_cursor_down(),
            kk::Action::CursorLeft => self.state.handle_cursor_left(),
            kk::Action::CursorRight => self.state.handle_cursor_right(),
            kk::Action::CursorLineStart => self.state.handle_cursor_line_start(),
            kk::Action::CursorLineEnd => self.state.handle_cursor_line_end(),
            kk::Action::CursorBufferStart => self.state.handle_cursor_buffer_start(),
            kk::Action::CursorBufferEnd => self.state.handle_cursor_buffer_end(),
            kk::Action::ViewRecenter => self.state.handle_view_recenter(),
            kk::Action::NewlineInsert => self.state.handle_newline_insert(),
            kk::Action::CharInsert => {
                if let tuinix::Input::Key(tuinix::KeyInput {
                    code: tuinix::KeyCode::Char(ch),
                    ..
                }) = input
                {
                    self.state.handle_char_insert(*ch);
                }
            }
            kk::Action::CharDeleteBackward => self.state.handle_char_delete_backward(),
            kk::Action::CharDeleteForward => self.state.handle_char_delete_forward(),
            kk::Action::LineDelete => self.state.handle_line_delete(),
            kk::Action::MarkSet => self.state.handle_mark_set(),
            kk::Action::MarkCut => self.state.handle_mark_cut(),
            kk::Action::ClipboardPaste => self.state.handle_clipboard_paste(),
            kk::Action::Grep(action) => {
                self.state.finish_editing();
                self.state.grep_mode = Some(kk::GrepMode::new(action));
                self.state.mark = None;
                self.state.set_message("Entered grep mode");
            }
            kk::Action::GrepNextHit => {
                if !self.state.highlight.items.is_empty() {
                    self.state.handle_grep_next_hit();
                } else {
                    self.state.set_message("No grep hits available");
                }
            }
            kk::Action::GrepPrevHit => {
                if !self.state.highlight.items.is_empty() {
                    self.state.handle_grep_prev_hit();
                } else {
                    self.state.set_message("No grep hits available");
                }
            }
        }

        Ok(())
    }

    /// Renders the buffer and writes it to `path` on behalf of the core.
    ///
    /// The core only produces the text; the write, and the dirty-flag update
    /// that follows a successful one, happen here.
    fn handle_buffer_save(&mut self) -> std::io::Result<()> {
        let text = self.state.handle_buffer_save();
        std::fs::write(&self.path, &text)?;
        self.state.mark_saved(text.chars().count());
        Ok(())
    }

    /// Reads `path` back and hands the text to the core to reload from.
    fn handle_buffer_reload(&mut self) -> std::io::Result<()> {
        let text = std::fs::read_to_string(&self.path)?;
        self.state.handle_buffer_reload(&text);
        Ok(())
    }

    fn text_area_region(&self) -> tuinix::Region {
        let footer_rows = if self.state.grep_mode.is_some() { 3 } else { 2 };
        self.driver.size().to_region().drop_bottom(footer_rows)
    }

    fn render(&mut self) -> std::io::Result<()> {
        let mut frame = tuinix::Frame::new(self.driver.size());

        let region = self.text_area_region();
        self.state.adjust_viewport(region.size);
        self.render_region(&mut frame, region, |frame| {
            self.text_area.render(&self.state, frame)
        });

        let mut frame_region = frame.size().to_region();
        let mut grep_region = frame_region;
        if self.state.grep_mode.is_some() {
            grep_region = frame_region.take_bottom(1);
            self.render_region(&mut frame, grep_region, |frame| {
                kk::GrepQueryRenderer.render(&self.state, frame)
            });
            frame_region = frame_region.drop_bottom(1);
        }

        let region = frame_region.take_bottom(2).take_top(1);
        let path = self.path.display().to_string();
        self.render_region(&mut frame, region, |frame| {
            self.status_line.render(&self.state, &path, frame)
        });

        let region = frame_region.take_bottom(1);
        self.render_region(&mut frame, region, |frame| {
            self.message_line.render(&self.state, frame)
        });

        if self.legend_visible {
            self.legend.render(self.context, &mut frame);
        }

        let cursor = if let Some(grep) = &self.state.grep_mode {
            Some(grep.cursor_position(grep_region))
        } else {
            Some(self.state.terminal_cursor_position())
        };

        let out = frame.render(self.prev_frame.as_ref(), cursor);
        self.driver.write_all(&out)?;
        self.driver.flush()?;
        self.prev_frame = Some(frame);

        self.state.message = None;
        Ok(())
    }

    fn render_region<F>(&self, frame: &mut tuinix::Frame, region: tuinix::Region, f: F)
    where
        F: FnOnce(&mut tuinix::Frame),
    {
        let mut sub_frame = tuinix::Frame::new(region.size);
        f(&mut sub_frame);
        frame.put_frame(region.position, &sub_frame);
    }
}
