//! End-to-end hide-and-come-back of the legend under the cursor.
//!
//! The legend is painted after the text, so a cursor underneath it would be
//! hidden by it; instead the legend steps aside while the cursor is inside it.
//! That only makes sense end to end: the position the cursor is judged by is
//! the one the terminal would put it at, so these tests read the grid the real
//! binary paints.

mod e2e;

use e2e::{KkHarness, scratch_file};

/// A row that belongs to the edit legend and to nothing else on screen.
const LEGEND_ROW: &str = "C-l recenter";

/// How far right the cursor must be to fall under the legend at 80 columns.
/// The edit legend is 13 columns wide, so its left edge is column 67.
const UNDER_THE_LEGEND: u16 = 72;

/// One long line, so moving right is never blocked by the end of a line.
fn long_line() -> String {
    "x".repeat(200)
}

#[test]
fn the_legend_hides_while_the_cursor_is_under_it() {
    let path = scratch_file("legend_cursor.txt");
    std::fs::write(&path, format!("{}\n", long_line())).expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");
    kk.wait_until("legend visible at startup", |h| {
        h.screen_contains(LEGEND_ROW)
    });

    // Click far enough right that the cursor lands under the legend.
    kk.click(0, UNDER_THE_LEGEND);
    kk.wait_until("legend hidden under the cursor", |h| {
        !h.screen_contains(LEGEND_ROW)
    });

    // The line is still painted where the legend was, so the hide is a hide and
    // not a clipped frame.
    let row = kk.find_row("xxxx").expect("the line still on screen");
    assert!(
        row.chars().count() >= UNDER_THE_LEGEND as usize,
        "the text should keep its columns where the legend was: {row:?}"
    );

    // Click back to the left edge and the legend returns on its own, without
    // any Escape: the toggle was never touched.
    kk.click(0, 0);
    kk.wait_until("legend back after the cursor left it", |h| {
        h.screen_contains(LEGEND_ROW)
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn escape_still_toggles_the_legend_under_the_cursor() {
    let path = scratch_file("legend_cursor_toggle.txt");
    std::fs::write(&path, format!("{}\n", long_line())).expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");
    kk.wait_until("legend visible at startup", |h| {
        h.screen_contains(LEGEND_ROW)
    });

    // Ask for the legend to be hidden, then move the cursor under where it was.
    kk.send_escape();
    kk.wait_until("legend hidden by Escape", |h| {
        !h.screen_contains(LEGEND_ROW)
    });
    kk.click(0, UNDER_THE_LEGEND);

    // Asking for it back under the cursor shows nothing: the cursor still owns
    // that corner, and the legend would cover it.
    kk.send_escape();
    kk.wait_until("hidden legend stays hidden under the cursor", |h| {
        !h.screen_contains(LEGEND_ROW)
    });

    // Moving away honours the toggle: the legend is back without another Escape.
    kk.click(0, 0);
    kk.wait_until("legend back once the cursor left", |h| {
        h.screen_contains(LEGEND_ROW)
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
