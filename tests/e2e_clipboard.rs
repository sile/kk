//! End-to-end clipboard export: `C-x w` (copy) reaches the terminal as OSC 52.
//!
//! The unit tests check that `State` records a [`kk::ClipboardExport`]; this
//! checks the other half -- that the edge turns it into a sequence the terminal
//! emulator recognises as a clipboard write, carrying exactly the copied text.
//! Only a copy exports: a cut fills kk's own clipboard and asks the terminal for
//! nothing.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn a_copy_reaches_the_terminal_as_an_osc_52_write() {
    let path = scratch_file("clipboard_copy.txt");
    std::fs::write(&path, "hello world\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // Mark at the buffer start, move to the end of the line, and copy with
    // `C-x w`. The buffer is left alone, so this is a copy, not a cut.
    kk.send_ctrl(' ');
    kk.send_ctrl('e');
    kk.send_ctrl('x');
    kk.send_char('w');

    kk.wait_for_text("Copied");

    let requests = kk.child_requests();
    let exported: Vec<String> = requests
        .into_iter()
        .filter_map(|request| match request {
            termnix::ChildRequest::SetClipboard { text, append, .. } => {
                assert!(!append, "a copy replaces, it does not append");
                Some(String::from_utf8(text).expect("clipboard text is UTF-8"))
            }
            _ => None,
        })
        .collect();

    assert_eq!(exported, vec!["hello world".to_string()]);

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_cut_does_not_reach_the_terminal() {
    let path = scratch_file("clipboard_cut.txt");
    std::fs::write(&path, "hello world\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // `C-w` cuts the marked region; it fills kk's own clipboard but must not
    // ask the terminal for anything.
    kk.send_ctrl(' ');
    kk.send_ctrl('e');
    kk.send_ctrl('w');
    kk.wait_for_text("Cut");

    let requests = kk.child_requests();
    assert!(
        !requests
            .iter()
            .any(|r| matches!(r, termnix::ChildRequest::SetClipboard { .. })),
        "a cut must not write to the terminal's clipboard: {requests:?}"
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
