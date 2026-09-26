//! End-to-end hide-and-come-back of the legend under the buffer cursor while a
//! search prompt is open.
//!
//! While a prompt is open the terminal cursor moves into the query, but the
//! buffer cursor is still painted in the text area, reversed, and a legend over
//! it would cover it. The legend therefore judges both the same way: it steps
//! aside for the buffer cursor too.

mod e2e;

use e2e::{KkHarness, scratch_file};

/// A row that belongs to the search legend and to nothing else on screen.
const LEGEND_ROW: &str = "C-d \u{2326} delete";

/// One long line, long enough that a click near the right edge lands under the
/// legend at 80 columns, where the search legend starts at column 67.
fn long_line() -> String {
    "x".repeat(200)
}

/// How far right a click must be to land under the legend at 80 columns.
const UNDER_THE_LEGEND: u16 = 72;

#[test]
fn the_legend_hides_for_the_buffer_cursor_while_the_prompt_is_open() {
    let path = scratch_file("legend_cursor_search.txt");
    std::fs::write(&path, format!("{}\n", long_line())).expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // A prompt is open for the whole test, so the legend on screen is the
    // search legend and the terminal cursor sits in the query.
    kk.send_ctrl('s');
    kk.wait_until("query prompt and search legend", |h| {
        h.screen_text().contains("Search:") && h.screen_contains(LEGEND_ROW)
    });

    // A click moves the buffer cursor while the prompt stays open, which is the
    // state this test is about: the buffer cursor is painted in the text area
    // while the terminal cursor is down in the query.
    kk.click(0, UNDER_THE_LEGEND);
    kk.wait_until("legend hidden under the buffer cursor", |h| {
        !h.screen_contains(LEGEND_ROW)
    });

    // The prompt is still on screen: the terminal cursor is in the query, only
    // the legend stepped aside.
    assert!(
        kk.screen_text().contains("Search:"),
        "the prompt should still be open:\n{}",
        kk.screen_text()
    );

    // Moving the buffer cursor back out brings the legend back, with the prompt
    // still open.
    kk.click(0, 0);
    kk.wait_until("legend back with the prompt still open", |h| {
        h.screen_contains(LEGEND_ROW) && h.screen_text().contains("Search:")
    });

    // `C-c` is the search context's kill-query, so the prompt has to be closed
    // before the usual quit chord means quit again.
    kk.send_ctrl('g');
    kk.wait_until("back in edit", |h| !h.screen_text().contains("Search:"));
    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
