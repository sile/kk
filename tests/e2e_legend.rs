//! End-to-end toggling of the legend with Escape.
//!
//! A lone `ESC` byte is ambiguous to the decoder, so `kk` waits a short
//! timeout before committing it as Escape. These tests therefore wait on the
//! visible effect rather than on a fixed sleep.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn escape_toggles_the_legend() {
    let path = scratch_file("legend.txt");
    std::fs::write(&path, "body\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // The legend is visible on startup; a row unique to it is the marker.
    kk.wait_until("legend visible at startup", |h| {
        h.screen_contains("C-j newline")
    });

    // Escape hides it.
    kk.send_escape();
    kk.wait_until("legend hidden after Escape", |h| {
        !h.screen_contains("C-j newline")
    });

    // The buffer is still there with the legend gone.
    assert!(
        kk.screen_contains("body"),
        "the buffer should remain visible:\n{}",
        kk.screen_text()
    );

    // Escape shows it again.
    kk.send_escape();
    kk.wait_until("legend visible after a second Escape", |h| {
        h.screen_contains("C-j newline")
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
