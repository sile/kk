//! End-to-end editing: type into the real binary and check the file on disk.
//!
//! These tests assert on the file contents rather than only the screen, because
//! "the buffer was written" is the editor's actual contract; the grid is how a
//! human sees it, not what the program promises.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn typing_then_saving_writes_the_edited_content() {
    let path = scratch_file("save_edit.txt");
    std::fs::write(&path, "one\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // Move to the end of the first line and append a second line.
    kk.send_ctrl('e');
    kk.send_key(termnix::KeyCode::Enter, termnix::Modifiers::new());
    kk.send_text("two");
    // Save is in the Ext context: `C-x` enters it, `C-s` saves.
    kk.send_ctrl('x');
    kk.send_ctrl('s');

    kk.wait_until("save message", |h| h.screen_contains("Saved"));

    let written = std::fs::read_to_string(&path).expect("read saved file");
    assert_eq!(written, "one\ntwo\n");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn saving_reports_the_character_count() {
    let path = scratch_file("save_count.txt");
    std::fs::write(&path, "abc\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    kk.send_ctrl('x');
    kk.send_ctrl('s');
    // "abc\n" is four characters; the status message names the count.
    kk.wait_for_text("Saved 4 chars");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn create_new_starts_empty_and_saves_what_is_typed() {
    let path = scratch_file("created.txt");
    assert!(!path.exists(), "scratch file should not exist yet");

    let mut kk = KkHarness::create_new(&path);
    kk.wait_for_text("Created");

    kk.send_text("fresh");
    kk.send_ctrl('x');
    kk.send_ctrl('s');
    kk.wait_until("create-new save message", |h| h.screen_contains("Saved"));

    let written = std::fs::read_to_string(&path).expect("read created file");
    // An empty buffer is a single blank line, so saving it appends a newline.
    assert_eq!(written, "fresh\n");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
