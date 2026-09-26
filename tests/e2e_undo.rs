//! End-to-end undo: press `C-u` right after typing and check the screen.
//!
//! This is the case a unit test can miss: a real session does not close the
//! edit run before the user hits `C-u`, so the first undo has to drop the run
//! that is still open.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn undo_right_after_typing_removes_what_was_typed() {
    let path = scratch_file("undo_after_typing.txt");
    std::fs::write(&path, "one\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // Type at the end of the line, then undo at once, with no cursor move in
    // between to close the edit run.
    kk.send_ctrl('e');
    kk.send_text("xyz");
    kk.wait_for_text("xyz");

    kk.send_ctrl('u');
    kk.wait_until("the typed text is gone", |h| !h.screen_contains("xyz"));

    // The file on disk is untouched until a save, but the buffer must be back
    // to what the file holds.
    let on_disk = std::fs::read_to_string(&path).expect("read scratch file");
    assert_eq!(on_disk, "one\n");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
