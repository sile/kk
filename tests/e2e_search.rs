//! End-to-end search and resize behavior of the real binary.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn search_enters_search_mode_and_finds_a_later_match() {
    let path = scratch_file("search.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // `C-s` in Edit starts a forward search and opens the query prompt, which
    // shares the message line rather than taking a row of its own.
    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_text().contains("Search:"));

    // Type the query, one character at a time.
    kk.send_text("gamma");

    // Typing finds the match but does not move the cursor to it: the status
    // line counts the one `gamma` in the buffer, and the cursor is still before
    // it, so `0/1`.
    kk.wait_until("query visible", |h| {
        h.screen_text().contains("Search: gamma") && h.screen_text().contains("0/1")
    });

    // `C-s` in Search is what jumps to the hit, which puts the cursor on it.
    kk.send_ctrl('s');
    kk.wait_until("cursor on the hit", |h| h.screen_text().contains("1/1"));

    // `Enter` accepts the search and returns to editing at the match.
    kk.send_key(termnix::KeyCode::Enter, termnix::Modifiers::new());

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn search_can_be_cancelled_with_ctrl_g() {
    let path = scratch_file("search_cancel.txt");
    std::fs::write(&path, "alpha\nbeta\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));

    kk.send_ctrl('g');

    // Cancelling leaves the prompt behind and returns to the buffer.
    kk.wait_until("search mode left", |h| !h.screen_contains("Search:"));

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn resizing_repaints_at_the_new_size() {
    let path = scratch_file("resize.txt");
    std::fs::write(&path, "hello\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    kk.resize(40, 100);

    // After the resize the content is still painted; the grid is wider now.
    kk.wait_for_text("hello");
    let rows = kk.screen_rows();
    assert!(
        rows.iter().any(|r| r.len() > 80),
        "expected a row wider than the original 80 columns:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
