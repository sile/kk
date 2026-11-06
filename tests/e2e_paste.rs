//! End-to-end paste: what reaches `kk` when nothing brackets the paste.
//!
//! `kk` never asks for bracketed-paste mode, so a paste arrives as the keys the
//! text spells. This test writes those bytes straight into the pty, which is
//! exactly what a terminal that never entered the mode sends.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn a_multi_line_paste_types_its_lines_in() {
    let path = scratch_file("paste_plain.txt");
    std::fs::write(&path, "one\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);

    // End of the first line, then a paste whose newline must break the line
    // rather than leave a literal one in the text.
    kk.send_ctrl('e');
    kk.send_raw(b"a\nb");
    kk.wait_for_text("b");

    kk.send_key(termnix::KeyCode::Tab, termnix::Modifiers::new());
    kk.wait_until("save message", |h| h.screen_contains("Saved"));

    let written = std::fs::read_to_string(&path).expect("read saved file");
    assert_eq!(written, "onea\nb\n");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
