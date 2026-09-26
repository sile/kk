//! End-to-end tests for the `:LINE[:COLUMN]` suffix on the FILE argument.
//!
//! The suffix is part of the positional argument, so these tests drive the real
//! binary with the bytes a shell would pass and read the cursor's position back
//! off the status line, which is painted by the same code path a user sees.

mod e2e;

use e2e::scratch_file;

/// Widens the terminal so the scratch path does not push `:LINE:COLUMN]` off
/// the status line. The default 80 columns are not enough for both.
fn widen(kk: &mut e2e::KkHarness) {
    kk.resize(24, 200);
}

/// A scratch file with four numbered lines.
fn write_lines(name: &str) -> std::path::PathBuf {
    let path = scratch_file(name);
    std::fs::write(&path, "one\ntwo\nthree\nfour\n").expect("write scratch file");
    path
}

#[test]
fn a_line_suffix_moves_the_cursor_to_that_line() {
    let path = write_lines("startup_line.txt");

    let mut kk = e2e::KkHarness::open_arg(&format!("{}:3", path.display()));
    widen(&mut kk);

    // Line 3 is `three`, 1-based as the command line spells it, and a line on
    // its own puts the cursor at the line's start.
    kk.wait_for_text(":3:1]");

    assert!(
        kk.screen_contains("three"),
        "expected the third line on screen:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_line_and_column_suffix_moves_the_cursor_to_both() {
    let path = write_lines("startup_line_column.txt");

    let mut kk = e2e::KkHarness::open_arg(&format!("{}:2:2", path.display()));
    widen(&mut kk);

    kk.wait_for_text(":2:2]");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn an_out_of_range_suffix_is_clamped_to_the_buffer_end() {
    let path = write_lines("startup_clamped.txt");

    let mut kk = e2e::KkHarness::open_arg(&format!("{}:9999", path.display()));
    widen(&mut kk);

    // The last line is line 4; the line after it is where the core clamps.
    kk.wait_for_text(":5:1]");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_path_without_a_suffix_still_starts_at_the_top() {
    let path = write_lines("startup_plain.txt");

    let mut kk = e2e::KkHarness::open(&path);
    widen(&mut kk);

    kk.wait_for_text(":1:1]");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
