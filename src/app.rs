use std::path::PathBuf;

use crate::error::Result;
use tuinix::{Terminal, TerminalEvent, TerminalInput, TerminalRegion};

use crate::{
    action::Action,
    binding::{Bindings, Context},
    grep_mode::{GrepMode, GrepQueryRenderer, Highlight},
    message_line::MessageLineRenderer,
    state::State,
    status_line::StatusLineRenderer,
    terminal::UnicodeTerminalFrame as TerminalFrame,
    text_area::TextAreaRenderer,
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
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
        let terminal = Terminal::new()?;
        Ok(Self {
            terminal,
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

        while !self.exit {
            self.render()?;

            match self.terminal.poll_event(&[], &[], None)? {
                Some(TerminalEvent::Input(input)) => {
                    self.handle_input(input)?;

                    // Handle buffered events before rendering
                    let timeout = std::time::Duration::ZERO;
                    while let Some(TerminalEvent::Input(input)) = self
                        .terminal
                        .poll_event(&[], &[], Some(timeout))
                        ?
                    {
                        self.handle_input(input)?;
                    }
                }
                Some(TerminalEvent::Resize(_size)) => {}
                Some(TerminalEvent::FdReady { .. }) => {
                    unreachable!()
                }
                None => {}
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, input: TerminalInput) -> Result<()> {
        let Some(binding) = self
            .bindings
            .get(self.context)
            .iter()
            .find(|b| b.matches(input))
        else {
            self.state
                .set_message(format!("No action found: '{}'", crate::fmt::input(input)));
            return Ok(());
        };

        let next_context = binding.context;
        let action = binding.action.clone();

        if let Some(action) = action {
            self.handle_action(action, input)?;
        }

        if let Some(context) = next_context {
            self.context = context;
        }

        Ok(())
    }

    fn handle_action(&mut self, action: Action, input: TerminalInput) -> Result<()> {
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
                if let TerminalInput::Key(key) = input {
                    self.state.handle_char_insert(key);
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

    fn text_area_region(&self) -> TerminalRegion {
        let footer_rows = if self.state.grep_mode.is_some() { 3 } else { 2 };
        self.terminal.size().to_region().drop_bottom(footer_rows)
    }

    fn render(&mut self) -> Result<()> {
        let mut frame = TerminalFrame::new(self.terminal.size());

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

        if let Some(grep) = &self.state.grep_mode {
            self.terminal
                .set_cursor(Some(grep.cursor_position(grep_region)));
        } else {
            self.terminal
                .set_cursor(Some(self.state.terminal_cursor_position()));
        }
        self.terminal.draw(frame)?;

        self.state.message = None;
        Ok(())
    }

    fn render_region<F>(
        &self,
        frame: &mut TerminalFrame,
        region: TerminalRegion,
        f: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut TerminalFrame) -> Result<()>,
    {
        let mut sub_frame = TerminalFrame::new(region.size);
        f(&mut sub_frame)?;
        frame.draw(region.position, &sub_frame);
        Ok(())
    }
}
