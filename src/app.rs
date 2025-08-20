use std::path::PathBuf;

use mame::{KeyMatcher as KeyPattern, UnicodeTerminalFrame as TerminalFrame};
use orfail::OrFail;
use tuinix::{KeyInput, Terminal, TerminalEvent, TerminalInput, TerminalRegion};

use crate::{
    action::Action,
    anchor::CursorAnchorLog,
    config::Config,
    grep_mode::{GrepMode, GrepQueryRenderer, Highlight},
    legend::LegendRenderer,
    message_line::MessageLineRenderer,
    state::State,
    status_line::StatusLineRenderer,
    text_area::TextAreaRenderer,
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    config: Config,
    state: State,
    anchor_log: CursorAnchorLog,
    text_area: TextAreaRenderer,
    message_line: MessageLineRenderer,
    status_line: StatusLineRenderer,
    legend: LegendRenderer,
    exit: bool,
}

impl App {
    pub fn new(path: PathBuf) -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            state: State::new(path).or_fail()?,
            anchor_log: CursorAnchorLog::default(),
            config: Config::load_str("<DEFAULT>", include_str!("../config.jsonc")).or_fail()?,
            text_area: TextAreaRenderer,
            message_line: MessageLineRenderer,
            status_line: StatusLineRenderer,
            legend: LegendRenderer,
            exit: false,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.state.set_message("Started");

        while !self.exit {
            self.render().or_fail()?;

            match self.terminal.poll_event(&[], &[], None).or_fail()? {
                Some(TerminalEvent::Input(input)) => {
                    self.handle_terminal_input(input).or_fail()?;

                    // Handle buffered events before rendering
                    let timeout = std::time::Duration::ZERO;
                    while let Some(TerminalEvent::Input(input)) = self
                        .terminal
                        .poll_event(&[], &[], Some(timeout))
                        .or_fail()?
                    {
                        self.handle_terminal_input(input).or_fail()?;
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

    fn handle_terminal_input(&mut self, input: TerminalInput) -> orfail::Result<()> {
        let TerminalInput::Key(key) = input else {
            unreachable!()
        };
        self.handle_key_input(key).or_fail()?;
        Ok(())
    }

    fn handle_key_input(&mut self, key: KeyInput) -> orfail::Result<()> {
        let Some(binding) = self.config.current_keymap().get_binding(key) else {
            self.state
                .set_message(format!("No action found: '{}'", mame::display_key(key)));
            return Ok(());
        };

        // TODO: remove clone
        for action in binding.actions.clone() {
            self.handle_action(action, key).or_fail()?;
        }

        Ok(())
    }

    fn handle_action(&mut self, action: Action, key: KeyInput) -> orfail::Result<()> {
        match action {
            Action::Multiple(actions) => {
                for action in actions {
                    self.handle_action(action, key).or_fail()?;
                }
            }
            Action::Quit => {
                self.exit = true;
            }
            Action::Cancel => {
                self.state.mark = None;
                if let Some(grep) = self.state.grep_mode.take() {
                    grep.save_query().or_fail()?;
                }
                self.state.highlight = Highlight::default();
                self.state.context.enter("__main__");
                self.state.set_message("Canceled");
            }
            Action::BufferSave => self.state.handle_buffer_save().or_fail()?,
            Action::BufferReload => self.state.handle_buffer_reload().or_fail()?,
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
            Action::CharInsert => self.state.handle_char_insert(key),
            Action::CharDeleteBackward => self.state.handle_char_delete_backward(),
            Action::CharDeleteForward => self.state.handle_char_delete_forward(),
            Action::LineDelete => self.state.handle_line_delete().or_fail()?,
            Action::MarkSet => self.state.handle_mark_set(),
            Action::MarkCopy => self.state.handle_mark_copy().or_fail()?,
            Action::MarkCut => self.state.handle_mark_cut().or_fail()?,
            Action::ClipboardPaste => self.state.handle_clipboard_paste().or_fail()?,
            Action::ShellCommand(action) => {
                self.state.handle_external_command(&action).or_fail()?
            }
            Action::CursorAnchor => {
                let anchor = self.state.current_cursor_anchor();
                self.state.set_message(format!("Anchor: {anchor}"));
                self.anchor_log.append(anchor).or_fail()?;
            }
            Action::CursorJump => {
                let current = self.state.current_cursor_anchor();
                if let Some(anchor) = self.anchor_log.prev_anchor(&current).or_fail()? {
                    self.state.restore_anchor(&anchor).or_fail()?;
                    self.state.set_message(format!("Jump: {anchor}"));
                }
            }
            Action::ContextSet(c) => {
                self.state.context.enter(&c.name);
                self.state.set_message(format!("New context: {}", c.name));
            }
            Action::Echo(m) => {
                self.state.set_message(&m.message);
            }
            Action::Grep(action) => {
                self.state.finish_editing();
                self.state.grep_mode = Some(GrepMode::new(action));
                self.state.mark = None;
                self.state.context.enter("__grep__");
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
            Action::GrepNextQuery => {
                self.state.handle_grep_next_query();
            }
            Action::GrepPrevQuery => {
                self.state.handle_grep_prev_query();
            }
            Action::GotoLine => self.state.handle_goto_line().or_fail()?,
            Action::CursorLeftSkipChars(c) => self.state.handle_cursor_left_skip_chars(&c.chars),
            Action::CursorRightSkipChars(c) => self.state.handle_cursor_right_skip_chars(&c.chars),
            Action::GrepReplaceHit => self.state.handle_grep_replace_hit().or_fail()?,
        }
        Ok(())
    }

    fn text_area_region(&self) -> TerminalRegion {
        let footer_rows = if self.state.grep_mode.is_some() { 3 } else { 2 };
        self.terminal.size().to_region().drop_bottom(footer_rows)
    }

    fn render(&mut self) -> orfail::Result<()> {
        let mut frame = TerminalFrame::new(self.terminal.size());

        let region = self.text_area_region();
        self.state.adjust_viewport(region.size);
        self.render_region(&mut frame, region, |frame| {
            self.text_area.render(&self.state, frame).or_fail()
        })?;

        let mut frame_region = frame.size().to_region();
        let mut grep_region = frame_region;
        if self.state.grep_mode.is_some() {
            grep_region = frame_region.take_bottom(1);
            self.render_region(&mut frame, grep_region, |frame| {
                GrepQueryRenderer.render(&self.state, frame).or_fail()
            })?;
            frame_region = frame_region.drop_bottom(1);
        }

        let region = frame_region.take_bottom(2).take_top(1);
        self.render_region(&mut frame, region, |frame| {
            self.status_line.render(&self.state, frame).or_fail()
        })?;

        let region = frame_region.take_bottom(1);
        self.render_region(&mut frame, region, |frame| {
            self.message_line.render(&self.state, frame).or_fail()
        })?;

        let region = self.legend.region(&self.config, &self.state, frame.size());
        self.render_region(&mut frame, region, |frame| {
            self.legend
                .render(&self.config, &self.state, frame)
                .or_fail()
        })?;

        if let Some(grep) = &self.state.grep_mode {
            self.terminal
                .set_cursor(Some(grep.cursor_position(grep_region)));
        } else {
            self.terminal
                .set_cursor(Some(self.state.terminal_cursor_position()));
        }
        self.terminal.draw(frame).or_fail()?;

        self.state.message = None;
        Ok(())
    }

    fn render_region<F>(
        &self,
        frame: &mut TerminalFrame,
        region: TerminalRegion,
        f: F,
    ) -> orfail::Result<()>
    where
        F: FnOnce(&mut TerminalFrame) -> orfail::Result<()>,
    {
        let mut sub_frame = TerminalFrame::new(region.size);
        f(&mut sub_frame).or_fail()?;
        frame.draw(region.position, &sub_frame);
        Ok(())
    }
}
