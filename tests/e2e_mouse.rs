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
    kk.wait_for_text("Opened");

    // Widened so the status line's tail is on one row, as below.
    kk.resize(24, 200);

    // Click on row 2, column 0: that is the line `three`.
    kk.click(2, 0);

    // The status line reports the cursor position as `ROW(ROWS):COL(COLS)`.
    kk.wait_for_text(":3(5):1(5)");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_click_past_the_line_end_lands_on_the_line_end() {
    let path = scratch_file("mouse_click_end.txt");
    std::fs::write(&path, TEXT).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // The status line echoes the path, which is long; widen the terminal so the
    // whole `ROW(ROWS):COL(COLS)` tail fits on one row.
    kk.resize(24, 200);

    // Row 1 is `two`; clicking far to its right clamps to its end.
    kk.click(1, 60);

    kk.wait_for_text(":2(5):4(3)");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_click_on_the_message_line_does_not_move_the_cursor() {
    let path = scratch_file("mouse_click_outside.txt");
    std::fs::write(&path, TEXT).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // Widened so the status line's tail is on one row, as above.
    kk.resize(24, 200);

    // The message line is the bottom row of a 24-row terminal; a click there is
    // outside the text area and is ignored.
    kk.click(23, 5);

    // The cursor is still at the origin, which the status line shows.
    kk.wait_for_text(":1(5):1(3)");
    assert!(
        !kk.screen_contains("No action found"),
        "an ignored click should stay silent:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn the_wheel_scrolls_the_cursor_and_the_view() {
    // Ten lines in a terminal that shows far fewer than that at once, so the
    // view has to move for the cursor to stay visible.
    let path = scratch_file("mouse_wheel.txt");
    let text: String = (1..=10).map(|n| format!("line{n}\n")).collect();
    std::fs::write(&path, &text).expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // Shrink the terminal so the whole buffer does not fit and the view must
    // scroll to keep the cursor visible. It is kept wide so the status line's
    // tail, which is what these assertions read, stays on one row.
    kk.resize(8, 200);
    kk.wait_for_text(":1(10):1(");

    // One notch is three lines, so the cursor lands on line 4.
    kk.scroll_down(0, 0);
    kk.wait_for_text(":4(10):1(");
    assert!(
        kk.screen_contains("line4"),
        "the view should follow the cursor:\n{}",
        kk.screen_text()
    );

    // Scrolling back up returns the cursor and the view to the start.
    kk.scroll_up(0, 0);
    kk.wait_for_text(":1(10):1(");
    assert!(
        kk.screen_contains("line1"),
        "the view should follow the cursor back:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
