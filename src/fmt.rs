//! Formatting utilities for terminal UI display elements.

/// Creates a displayable representation of a terminal input (key or mouse).
pub fn input(input: &tuinix::Input) -> String {
    match input {
        tuinix::Input::Key(key) => key_input(key),
        tuinix::Input::Mouse(mouse) => mouse_input(mouse.kind),
        tuinix::Input::Unrecognized { .. } => "<UNRECOGNIZED>".to_owned(),
        tuinix::Input::Paste { .. } => "<PASTE>".to_owned(),
    }
}

/// Renders a key chord as its `C-` spelling, with special keys in angle
/// brackets and control characters in hex.
///
/// Alt is a modifier `kk` does not act on, so it is not part of the spelling:
/// an Alt chord renders as the same chord without Alt.
fn key_input(key: &tuinix::KeyInput) -> String {
    let mut out = String::new();
    if key.ctrl {
        out.push_str("C-");
    }

    match key.code {
        tuinix::KeyCode::Up => out.push_str("<UP>"),
        tuinix::KeyCode::Down => out.push_str("<DOWN>"),
        tuinix::KeyCode::Left => out.push_str("<LEFT>"),
        tuinix::KeyCode::Right => out.push_str("<RIGHT>"),
        tuinix::KeyCode::Enter => out.push_str("<ENTER>"),
        tuinix::KeyCode::Escape => out.push_str("<ESCAPE>"),
        tuinix::KeyCode::Backspace => out.push_str("<BACKSPACE>"),
        tuinix::KeyCode::Tab => out.push_str("<TAB>"),
        tuinix::KeyCode::BackTab => out.push_str("<BACKTAB>"),
        tuinix::KeyCode::Delete => out.push_str("<DELETE>"),
        tuinix::KeyCode::Insert => out.push_str("<INSERT>"),
        tuinix::KeyCode::Home => out.push_str("<HOME>"),
        tuinix::KeyCode::End => out.push_str("<END>"),
        tuinix::KeyCode::PageUp => out.push_str("<PAGEUP>"),
        tuinix::KeyCode::PageDown => out.push_str("<PAGEDOWN>"),
        tuinix::KeyCode::F(n) => out.push_str(&format!("<F{n}>")),
        tuinix::KeyCode::Char(ch) if ch.is_control() => out.push_str(&format!("0x{:x}", ch as u32)),
        tuinix::KeyCode::Char(ch) => out.push(ch),
    }

    out
}

/// Renders a mouse event in angle brackets.
fn mouse_input(kind: tuinix::MouseInputKind) -> String {
    match kind {
        tuinix::MouseInputKind::LeftPress => "<LEFTCLICK>",
        tuinix::MouseInputKind::LeftRelease => "<LEFTRELEASE>",
        tuinix::MouseInputKind::RightPress => "<RIGHTCLICK>",
        tuinix::MouseInputKind::RightRelease => "<RIGHTRELEASE>",
        tuinix::MouseInputKind::MiddlePress => "<MIDDLECLICK>",
        tuinix::MouseInputKind::MiddleRelease => "<MIDDLERELEASE>",
        tuinix::MouseInputKind::Drag => "<DRAG>",
        tuinix::MouseInputKind::ScrollUp => "<SCROLLUP>",
        tuinix::MouseInputKind::ScrollDown => "<SCROLLDOWN>",
    }
    .to_owned()
}
