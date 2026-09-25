//! End-to-end smoke test: the real `kk` binary starts, paints, and quits.
//!
//! This is the smallest test that proves the harness works at all. If the
//! binary cannot take over a PTY, or the emulator cannot read its output back,
//! this test fails and every other test in the suite is meaningless.

mod e2e;

use e2e::scratch_file;

#[test]
fn kk_starts_paints_and_quits() {
    let path = scratch_file("smoke.txt");
    std::fs::write(&path, "hello\n").expect("write scratch file");

    let mut kk = e2e::KkHarness::open(&path);

    // The first message says the file was opened.
    kk.wait_for_text("Opened");

    // The buffer's content is on screen.
    assert!(
        kk.screen_contains("hello"),
        "expected the file content on screen:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
