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

/// How many lines one wheel notch scrolls.
const SCROLL_ROWS: isize = 3;

/// Owns the terminal edge and drives the core until the user quits.
#[derive(Debug)]
pub struct App {
    driver: tuinix::TerminalDriver,
    input: tuinix::InputDecoder,
    prev_frame: Option<tuinix::Frame>,
    path: PathBuf,
    saved_text: String,
    mode: kk::Mode,
    state: kk::State,
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
    ///
    /// The text read (empty under `--create-new`) is remembered as what this
    /// edge has last seen, so a later save can tell whether the file changed
    /// underneath it.
    ///
    /// `position` is where to leave the cursor, 0-based; an out-of-range
    /// position is clamped by the core rather than rejected. The cursor is
    /// centered in the first frame, so a named line lands in the middle of the
    /// text area instead of against an edge.
    pub fn new<P: AsRef<Path>>(
        path: P,
        create_new: bool,
        position: tuinix::Position,
    ) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        // `create_new` leaves the buffer empty without a read: the call only
        // succeeds when it has just brought an empty file into existence.
        let text = if create_new {
            std::fs::File::create_new(&path)?;
            String::new()
        } else {
            std::fs::read_to_string(&path)?
        };

        let buffer = kk::TextBuffer::new(&text);

        let mut state = kk::State::new(buffer);
        state.handle_cursor_to_position(position.row, position.col);
        state.set_message(if create_new { "Created" } else { "Opened" });
        let mut driver = tuinix::TerminalDriver::new()?;
        // Mouse reporting is a convenience, not a requirement: a terminal that
        // refuses it (or a redirect that never asks for it) still edits fine, so
        // the failure is reported and swallowed rather than fatal.
        if let Err(e) = driver.enable_mouse_reporting() {
            state.set_message(format!("Mouse reporting unavailable: {e}"));
        }

        Ok(Self {
            driver,
            input: tuinix::InputDecoder::new(),
            prev_frame: None,
            path,
            saved_text: text,
            state,
            mode: kk::Mode::Edit,
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
        if let tuinix::Input::Mouse(mouse) = input {
            self.handle_mouse(mouse);
            return Ok(());
        }

        let Some(resolved) = self.mode.resolve(&input) else {
            self.state
                .set_message(format!("No action found: '{}'", kk::display_input(&input)));
            return Ok(());
        };

        if let Some(action) = resolved.action {
            self.handle_action(action, &input)?;
        }

        if let Some(mode) = resolved.mode {
            self.mode = mode;
        }

        Ok(())
    }

    /// Carries out a mouse event.
    ///
    /// Mouse input never goes through [`kk::resolve`]: the binding table maps a
    /// key to one action while a mouse event means different things at different
    /// positions, so the position is resolved here, against the text area's
    /// region, before any action is chosen. Anything else -- other buttons,
    /// presses on the status line or the legend, motion -- is ignored without a
    /// message, so a stray click cannot bury the message line.
    fn handle_mouse(&mut self, mouse: tuinix::MouseInput) {
        let region = self.text_area_region();
        let inside = region.contains(mouse.position);
        let text_area_rel = tuinix::Position {
            row: mouse.position.row.saturating_sub(region.position.row),
            col: mouse.position.col.saturating_sub(region.position.col),
        };

        match mouse.kind {
            tuinix::MouseInputKind::LeftPress if inside => self
                .state
                .handle_cursor_to_screen_position(text_area_rel.row, text_area_rel.col),
            tuinix::MouseInputKind::ScrollUp => self.state.handle_scroll(-SCROLL_ROWS),
            tuinix::MouseInputKind::ScrollDown => self.state.handle_scroll(SCROLL_ROWS),
            _ => {}
        }
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
            kk::Action::Cancel => self.state.handle_cancel(),
            kk::Action::BufferSave => {
                self.handle_buffer_save(true)?;
                self.state.clear_transient_state();
            }
            kk::Action::BufferForceSave => {
                self.handle_buffer_save(false)?;
                self.state.clear_transient_state();
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
            kk::Action::LineCutTail => self.state.handle_line_cut_tail(),
            kk::Action::MarkSet => self.state.handle_mark_set(),
            kk::Action::MarkCut => self.state.handle_mark_cut(),
            kk::Action::ClipboardPaste => self.state.handle_clipboard_paste(),
            kk::Action::ExtEnter => self.state.handle_ext_enter(),
            kk::Action::SearchEnter => self.state.handle_search_enter(),
            kk::Action::SearchCancel => self.state.handle_search_cancel(),
            kk::Action::SearchFinish => self.state.handle_search_finish(),
            kk::Action::SearchCutQuery => self.state.handle_search_cut_query(),
            kk::Action::SearchNextHit => self.state.handle_search_next_hit(),
            kk::Action::SearchPrevHit => self.state.handle_search_prev_hit(),
        }

        Ok(())
    }

    /// Renders the buffer and writes it to `path` on behalf of the core.
    ///
    /// The core only produces the text; the write, and the report that follows
    /// a successful one, happen here. When `check_disk` is set the file is read
    /// back first and a write is refused when it no longer holds what this edge
    /// last read or wrote, so another writer's version is never silently lost;
    /// the refusal names the chord that saves anyway.
    fn handle_buffer_save(&mut self, check_disk: bool) -> std::io::Result<()> {
        let text = self.state.handle_buffer_save();
        if check_disk {
            match std::fs::read_to_string(&self.path) {
                Ok(disk) if disk != self.saved_text => {
                    self.state
                        .set_message("Changed on disk; C-x s to overwrite");
                    return Ok(());
                }
                // A file that has gone missing is not what this edge last saw,
                // so it counts as a change too: writing would recreate it.
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    self.state.set_message("File is gone; C-x s to overwrite");
                    return Ok(());
                }
                Err(e) => return Err(e),
            }
        }
        std::fs::write(&self.path, &text)?;
        self.saved_text = text;
        self.state.report_saved(self.saved_text.chars().count());
        Ok(())
    }

    /// Reads `path` back and hands the text to the core to reload from.
    fn handle_buffer_reload(&mut self) -> std::io::Result<()> {
        let text = std::fs::read_to_string(&self.path)?;
        self.saved_text = text.clone();
        self.state.handle_buffer_reload(&text);
        Ok(())
    }

    /// Returns the region the buffer text is painted into.
    ///
    /// The bottom two rows are the status line and the message line. A search
    /// prompt shares the message line rather than taking a row of its own, so
    /// the text area is the same height whether or not a search is open.
    fn text_area_region(&self) -> tuinix::Region {
        self.driver.size().to_region().drop_bottom(2)
    }

    /// Returns the region the legend of `mode` is painted into, or `None` when
    /// the frame is too small to hold it whole.
    ///
    /// The legend is not painted at all when it does not fit, so a `None` here
    /// means there is nothing for the cursor to share and nothing to hide.
    fn legend_region(&self, mode: kk::Mode) -> Option<tuinix::Region> {
        let size = self.driver.size();
        let legend = mode.legend_size(size);
        if legend != kk::full_legend_size(mode) {
            return None;
        }

        Some(tuinix::Region {
            position: tuinix::Position {
                row: 0,
                col: size.cols - legend.cols,
            },
            size: legend,
        })
    }

    fn render(&mut self) -> std::io::Result<()> {
        let mut frame = tuinix::Frame::new(self.driver.size());

        let region = self.text_area_region();
        self.state.adjust_viewport(region.size);
        self.render_region(&mut frame, region, |frame| {
            kk::render_text_area(&self.state, frame)
        });

        let frame_region = frame.size().to_region();

        let status_region = frame_region.take_bottom(2).take_top(1);
        let path = self.path.display().to_string();
        self.render_region(&mut frame, status_region, |frame| {
            kk::render_status_line(&self.state, &path, frame)
        });

        let message_region = frame_region.take_bottom(1);
        self.render_region(&mut frame, message_region, |frame| {
            kk::render_message_line(&self.state, frame)
        });

        // The terminal cursor is in the query while one is open, and on the
        // buffer's cursor otherwise; the query shares the message line, so that
        // is the region its position is measured in.
        let cursor = if let Some(search) = &self.state.search_prompt {
            Some(search.cursor_position(message_region))
        } else {
            Some(self.state.terminal_cursor_position())
        };

        // The legend is painted last so it sits above the text, but the cursor
        // is painted above everything, so a legend over the cursor would hide
        // it. It steps aside for as long as the cursor is inside it, and comes
        // back on its own: `legend_visible` is left alone, so the `Esc` toggle
        // still means what the user last asked for.
        //
        // The position judged is the buffer cursor's, not `cursor`: while a
        // prompt is open the terminal cursor is in the query, but the buffer
        // cursor is still painted in the text area, reversed, and that is the
        // one a legend over it would cover.
        if let Some(legend_region) = self.legend_region(self.mode) {
            let buffer_cursor = self.state.terminal_cursor_position();
            if self.legend_visible && !legend_region.contains(buffer_cursor) {
                kk::render_legend(self.mode, &mut frame);
            }
        }

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
