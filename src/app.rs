use std::io::{Read, Write};
use std::path::PathBuf;

use crate::error::Result;
use tuinix::{Frame, Input, InputDecoder, Region, TerminalDriver};

use crate::{
    action::Action,
    binding::{Bindings, Context},
    grep_mode::{GrepMode, GrepQueryRenderer, Highlight},
    message_line::MessageLineRenderer,
    state::State,
    status_line::StatusLineRenderer,
    text_area::TextAreaRenderer,
};

/// How long to wait for the rest of an escape sequence before a lone `ESC` byte
/// is treated as the Escape key.
const ESCAPE_TIMEOUT_MS: libc::c_int = 50;

#[derive(Debug)]
pub struct App {
    driver: TerminalDriver,
    input: InputDecoder,
    prev_frame: Option<Frame>,
    bindings: Bindings,
    context: Context,
    state: State,
    text_area: TextAreaRenderer,
    message_line: MessageLineRenderer,
    status_line: StatusLineRenderer,
    exit: bool,
}

impl App {
    pub fn new(path: PathBuf) -> Result<Self> {
        let driver = TerminalDriver::new()?;
        Ok(Self {
            driver,
            input: InputDecoder::new(),
            prev_frame: None,
            state: State::new(path)?,
            context: Context::Main,
            bindings: Bindings::new(),
            text_area: TextAreaRenderer,
            message_line: MessageLineRenderer,
            status_line: StatusLineRenderer,
            exit: false,
        })
    }

    pub fn run(mut self) -> Result<()> {
        self.state.set_message("Started");

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
            let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                // `poll` is never restarted by `SA_RESTART`, so SIGWINCH makes it
                // return `EINTR`; the resize byte is already in the pipe, so retry.
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err.into());
            }

            if fds
                .iter()
                .any(|fd| fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0)
            {
                return Err(std::io::Error::other("terminal closed").into());
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

    fn read_input(&mut self) -> Result<()> {
        let mut raw = [0u8; 256];
        loop {
            match self.driver.read(&mut raw) {
                Ok(n @ 1..) => self.input.feed(&raw[..n]),
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    fn handle_input(&mut self, input: Input) -> Result<()> {
        let Some(binding) = self
            .bindings
            .get(self.context)
            .iter()
            .find(|b| b.matches(&input))
        else {
            self.state
                .set_message(format!("No action found: '{}'", crate::fmt::input(&input)));
            return Ok(());
        };

        let next_context = binding.context;
        let action = binding.action.clone();

        if let Some(action) = action {
            self.handle_action(action, &input)?;
        }

        if let Some(context) = next_context {
            self.context = context;
        }

        Ok(())
    }

    fn handle_action(&mut self, action: Action, input: &Input) -> Result<()> {
        match action {
            Action::Multiple(actions) => {
                for action in actions {
                    self.handle_action(action, input)?;
                }
            }
            Action::Quit => {
                self.exit = true;
            }
            Action::Cancel => {
                self.state.mark = None;
                self.state.grep_mode = None;
                self.state.highlight = Highlight::default();
                self.state.set_message("Canceled");
            }
            Action::BufferSave => self.state.handle_buffer_save()?,
            Action::BufferReload => self.state.handle_buffer_reload()?,
            Action::BufferUndo => self.state.handle_buffer_undo(),
            Action::CursorUp => self.state.handle_cursor_up(),
            Action::CursorDown => self.state.handle_cursor_down(),
            Action::CursorLeft => self.state.handle_cursor_left(),
            Action::CursorRight => self.state.handle_cursor_right(),
            Action::CursorLineStart => self.state.handle_cursor_line_start(),
            Action::CursorLineEnd => self.state.handle_cursor_line_end(),
            Action::CursorBufferStart => self.state.handle_cursor_buffer_start(),
            Action::CursorBufferEnd => self.state.handle_cursor_buffer_end(),
            Action::CursorPageUp => {
                let text_area_size = self.text_area_region().size;
                self.state.handle_cursor_page_up(text_area_size);
            }
            Action::CursorPageDown => {
                let text_area_size = self.text_area_region().size;
                self.state.handle_cursor_page_down(text_area_size);
            }
            Action::CursorSkipSpaces => self.state.handle_cursor_skip_spaces(),
            Action::CursorUpSkipSpaces => self.state.handle_cursor_up_skip_spaces(),
            Action::CursorDownSkipSpaces => self.state.handle_cursor_down_skip_spaces(),
            Action::ViewRecenter => self.state.handle_view_recenter(),
            Action::NewlineInsert => self.state.handle_newline_insert(),
            Action::CharInsert => {
                if let Input::Key(key) = input {
                    self.state.handle_char_insert(*key);
                }
            }
            Action::CharDeleteBackward => self.state.handle_char_delete_backward(),
            Action::CharDeleteForward => self.state.handle_char_delete_forward(),
            Action::LineDelete => self.state.handle_line_delete()?,
            Action::MarkSet => self.state.handle_mark_set(),
            Action::MarkCopy => self.state.handle_mark_copy()?,
            Action::MarkCut => self.state.handle_mark_cut()?,
            Action::ClipboardPaste => self.state.handle_clipboard_paste()?,
            Action::Echo(m) => {
                self.state.set_message(&m.message);
            }
            Action::Grep(action) => {
                self.state.finish_editing();
                self.state.grep_mode = Some(GrepMode::new(action));
                self.state.mark = None;
                self.state.set_message("Entered grep mode");
            }
            Action::GrepNextHit => {
                if !self.state.highlight.items.is_empty() {
                    self.state.handle_grep_next_hit();
                } else {
                    self.state.set_message("No grep hits available");
                }
            }
            Action::GrepPrevHit => {
                if !self.state.highlight.items.is_empty() {
                    self.state.handle_grep_prev_hit();
                } else {
                    self.state.set_message("No grep hits available");
                }
            }
            Action::CursorLeftSkipChars(c) => self.state.handle_cursor_left_skip_chars(&c.chars),
            Action::CursorRightSkipChars(c) => self.state.handle_cursor_right_skip_chars(&c.chars),
        }
        Ok(())
    }

    fn text_area_region(&self) -> Region {
        let footer_rows = if self.state.grep_mode.is_some() { 3 } else { 2 };
        self.driver.size().to_region().drop_bottom(footer_rows)
    }

    fn render(&mut self) -> Result<()> {
        let mut frame = Frame::new(self.driver.size());

        let region = self.text_area_region();
        self.state.adjust_viewport(region.size);
        self.render_region(&mut frame, region, |frame| {
            self.text_area.render(&self.state, frame)
        })?;

        let mut frame_region = frame.size().to_region();
        let mut grep_region = frame_region;
        if self.state.grep_mode.is_some() {
            grep_region = frame_region.take_bottom(1);
            self.render_region(&mut frame, grep_region, |frame| {
                GrepQueryRenderer.render(&self.state, frame)
            })?;
            frame_region = frame_region.drop_bottom(1);
        }

        let region = frame_region.take_bottom(2).take_top(1);
        self.render_region(&mut frame, region, |frame| {
            self.status_line.render(&self.state, frame)
        })?;

        let region = frame_region.take_bottom(1);
        self.render_region(&mut frame, region, |frame| {
            self.message_line.render(&self.state, frame)
        })?;

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

    fn render_region<F>(&self, frame: &mut Frame, region: Region, f: F) -> Result<()>
    where
        F: FnOnce(&mut Frame) -> Result<()>,
    {
        let mut sub_frame = Frame::new(region.size);
        f(&mut sub_frame)?;
        frame.put_frame(region.position, &sub_frame);
        Ok(())
    }
}
