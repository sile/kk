//! End-to-end mouse tests: the real `kk` binary reacts to clicks and wheels.
//!
//! These drive the same bytes a terminal would send once `kk` has enabled mouse
//! reporting, so they cover the whole path: the mode is turned on, the decoder
//! parses the report, and the editor's edge moves the cursor from it.

mod e2e;

use e2e::scratch_file;

/// The text area starts at row 0, so a grid row is also a text area row.
/// Rows 1 and 2 of this buffer are `two` and `three`.
const TEXT: &str = "one\ntwo\nthree\nfour\nfive\n";

#[test]
fn a_click_moves_the_cursor_to_the_clicked_line() {
    let path = scratch_file("mouse_click.txt");
    std::fs::write(&path, TEXT).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);

    // Widened so the status line's tail is on one row, as below.
    kk.resize(24, 200);

    // Click on row 2, column 0: that is the line `three`.
    kk.click(2, 0);

    // The status line reports the cursor position as `ROW:COL`.
    kk.wait_for_text(":3:1]");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_click_past_the_line_end_lands_on_the_line_end() {
    let path = scratch_file("mouse_click_end.txt");
    std::fs::write(&path, TEXT).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);

    // The status line echoes the path, which is long; widen the terminal so the
    // whole `ROW:COL` tail fits on one row.
    kk.resize(24, 200);

    // Row 1 is `two`; clicking far to its right clamps to its end.
    kk.click(1, 60);

    kk.wait_for_text(":2:4]");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_click_on_the_message_line_does_not_move_the_cursor() {
    let path = scratch_file("mouse_click_outside.txt");
    std::fs::write(&path, TEXT).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);

    // Widened so the status line's tail is on one row, as above.
    kk.resize(24, 200);

    // The message line is the bottom row of a 24-row terminal; a click there is
    // outside the text area and is ignored.
    kk.click(23, 5);

    // The cursor is still at the origin, which the status line shows.
    kk.wait_for_text(":1:1]");
    assert!(
        !kk.screen_contains("No action found"),
        "an ignored click should stay silent:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn the_wheel_scrolls_the_view_and_the_cursor_rides_with_it() {
    // Forty lines in a terminal that shows far fewer than that at once, so the
    // view has room to move on its own.
    let path = scratch_file("mouse_wheel.txt");
    let text: String = (1..=40).map(|n| format!("line{n}\n")).collect();
    std::fs::write(&path, &text).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);

    // Shrink the terminal so the whole buffer does not fit. It is kept wide so
    // the status line's tail, which is what these assertions read, stays on one
    // row.
    kk.resize(8, 200);
    kk.wait_for_text(":1:1]");

    // Move the cursor down one row first, so it is not already on the viewport's
    // own row: the notch must move the text under it, not drag it from the top.
    kk.send_ctrl('n');
    kk.wait_until("the cursor on line 2", |h| h.screen_contains(":2:1]"));

    // One notch is three lines. The view moves by three, and the cursor keeps
    // its screen row, so it lands on line 5 while the viewport sits on line 4:
    // line 4 is now the top drawn row and line 2 has scrolled off.
    kk.scroll_down(0, 0);
    kk.wait_for_text(":5:1]");
    assert!(
        kk.screen_contains("line4"),
        "the view should have moved by the notch:\n{}",
        kk.screen_text()
    );

    // Scrolling back up returns the view and the cursor to the start.
    kk.scroll_up(0, 0);
    kk.wait_for_text(":2:1]");
    assert!(
        kk.screen_contains("line1"),
        "the view should scroll back to the top:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
