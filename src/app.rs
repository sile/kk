use std::path::PathBuf;

use orfail::OrFail;
use tuinix::{KeyInput, Terminal, TerminalEvent, TerminalInput, TerminalRegion};

use crate::{
    action::Action,
    config::Config,
    legend::LegendRenderer,
    mame::{KeyInputDisplay, TerminalFrame},
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
            config: Config::default(),
            text_area: TextAreaRenderer,
            message_line: MessageLineRenderer,
            status_line: StatusLineRenderer,
            legend: LegendRenderer,
            exit: false,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        let mut dirty = true;
        self.state.set_message("Started");

        while !self.exit {
            if dirty {
                self.render().or_fail()?;
                dirty = false;
            }

            match self.terminal.poll_event(None).or_fail()? {
                Some(TerminalEvent::Input(input)) => {
                    let TerminalInput::Key(key) = input;
                    self.handle_key_input(key).or_fail()?;
                    dirty = true;
                }
                Some(TerminalEvent::Resize(_size)) => {
                    dirty = true;
                }
                None => {}
            }
        }

        Ok(())
    }

    fn handle_key_input(&mut self, key: KeyInput) -> orfail::Result<()> {
        let Some(action_name) = self.config.keybindings.get(&self.state.context, key) else {
            self.state
                .set_message(format!("No action found: '{}'", KeyInputDisplay(key)));
            return Ok(());
        };

        let action = self.config.actions.get(action_name).or_fail()?;
        match action {
            Action::Quit => {
                self.exit = true;
            }
            Action::Cancel => {
                self.state.set_message("Canceled");
            }
            Action::BufferSave => self.state.handle_buffer_save().or_fail()?,
            Action::BufferReload => self.state.handle_buffer_reload().or_fail()?,
            Action::CursorUp => self.state.handle_cursor_up(),
            Action::CursorDown => self.state.handle_cursor_down(),
            Action::CursorLeft => self.state.handle_cursor_left(),
            Action::CursorRight => self.state.handle_cursor_right(),
            Action::CursorLineStart => self.state.handle_cursor_line_start(),
            Action::CursorLineEnd => self.state.handle_cursor_line_end(),
            Action::CursorBufferStart => self.state.handle_cursor_buffer_start(),
            Action::CursorBufferEnd => self.state.handle_cursor_buffer_end(),
            Action::CharDeleteBackward => self.state.handle_char_delete_backward(),
            Action::CharDeleteForward => self.state.handle_char_delete_forward(),
        }
        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        let mut frame = TerminalFrame::new(self.terminal.size());

        let region = frame.size().to_region().drop_bottom(2);
        self.state.adjust_viewport(region.size);
        self.render_region(&mut frame, region, |frame| {
            self.text_area.render(&self.state, frame).or_fail()
        })?;

        let region = frame.size().to_region().take_bottom(2).take_top(1);
        self.render_region(&mut frame, region, |frame| {
            self.status_line.render(&self.state, frame).or_fail()
        })?;

        let region = frame.size().to_region().take_bottom(1);
        self.render_region(&mut frame, region, |frame| {
            self.message_line.render(&self.state, frame).or_fail()
        })?;

        let region = self.legend.region(&self.config, &self.state, frame.size());
        self.render_region(&mut frame, region, |frame| {
            self.legend
                .render(&self.config, &self.state, frame)
                .or_fail()
        })?;

        self.terminal
            .set_cursor(Some(self.state.terminal_cursor_position()));
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
