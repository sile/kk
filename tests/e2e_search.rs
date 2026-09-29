//! End-to-end search and resize behavior of the real binary.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn search_enters_the_prompt_and_finds_a_later_match() {
    let path = scratch_file("search.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.wait_for_text("Opened");

    // `C-s` in Edit opens the query prompt, which shares the message line
    // rather than taking a row of its own. The direction is picked here, in the
    // prompt, by `C-s` and `C-r`, so the entry itself is directionless.
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
    kk.wait_until("the prompt left", |h| !h.screen_contains("Search:"));

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn cancelling_a_search_returns_the_cursor_to_where_it_started() {
    let path = scratch_file("search_cancel_cursor.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\ndelta\n").expect("write scratch file");

    // A wide terminal keeps the whole status line, and so the `:ROW:COL`
    // reading of the cursor, on screen whatever the scratch path is.
    let mut kk = KkHarness::open(&path);
    kk.resize(24, 200);
    kk.wait_for_text("Opened");
    kk.wait_until("cursor at the start", |h| h.screen_contains(":1:1]"));

    // Search for a word on a later row and jump to it, which moves the cursor
    // off row 1.
    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("gamma");
    kk.send_ctrl('s');
    kk.wait_until("cursor on the hit", |h| h.screen_contains(":3:1]"));

    // `C-g` abandons the search, so the cursor goes back to where the prompt
    // was opened rather than staying on the hit.
    kk.send_ctrl('g');
    kk.wait_until("cursor back at the start", |h| {
        !h.screen_contains("Search:") && h.screen_contains(":1:1]")
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn finishing_a_search_leaves_the_cursor_on_the_hit() {
    let path = scratch_file("search_finish_cursor.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\ndelta\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(24, 200);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("gamma");
    kk.send_ctrl('s');
    kk.wait_until("cursor on the hit", |h| h.screen_contains(":3:1]"));

    // `Enter` keeps the hit: editing resumes where the search landed.
    kk.send_key(termnix::KeyCode::Enter, termnix::Modifiers::new());
    kk.wait_until("editing on the hit", |h| {
        !h.screen_contains("Search:") && h.screen_contains(":3:1]")
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn cutting_from_the_query_shortens_it_without_touching_the_buffer() {
    let path = scratch_file("search_cut.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(24, 200);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("gamma");
    kk.wait_until("the query is typed", |h| h.screen_contains("Search: gamma"));

    // `C-k` in the prompt cuts from the query's cursor to its end. The cursor
    // is at the end, so the first one has nothing to take.
    kk.send_ctrl('k');
    kk.wait_until("the query is unchanged", |h| {
        h.screen_contains("Search: gamma")
    });

    // Move the query cursor back two characters, then cut the tail.
    kk.send_key(termnix::KeyCode::Left, termnix::Modifiers::new());
    kk.send_key(termnix::KeyCode::Left, termnix::Modifiers::new());
    kk.send_ctrl('k');
    kk.wait_until("the tail is gone", |h| {
        h.screen_contains("Search: gam") && !h.screen_contains("Search: gamma")
    });

    // The buffer is untouched: the first line still starts where it did.
    assert!(
        kk.screen_text().contains("alpha"),
        "the query edit never reached the buffer:\n{}",
        kk.screen_text()
    );

    // `C-c` is not bound in the prompt, so leave it before asking kk to quit.
    kk.send_ctrl('g');
    kk.wait_until("the prompt is left", |h| !h.screen_contains("Search:"));

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn the_gutter_shows_the_hits_while_searching_and_leaves_with_the_prompt() {
    let path = scratch_file("search_gutter.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(24, 200);
    kk.wait_for_text("Opened");

    // Before searching there is no gutter: the first line reads with no count
    // cell or separator in front of it.
    kk.wait_until("no gutter at rest", |h| {
        let text = h.screen_text();
        text.contains("alpha") && !text.contains("| alpha")
    });

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("gamma");

    // The gutter appears with the prompt: the line holding the one hit reads
    // `1 | gamma`, and the lines without a hit keep a blank count cell.
    kk.wait_until("gutter with the hit count", |h| {
        h.screen_text().contains("1 | gamma")
    });

    kk.send_ctrl('g');

    // Cancelling takes the gutter away again.
    kk.wait_until("gutter gone", |h| {
        let text = h.screen_text();
        !text.contains("Search:") && text.contains("gamma") && !text.contains("| gamma")
    });

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_jump_to_a_hit_past_the_edge_leaves_the_cursor_visible() {
    // Nineteen lines, the last of which is the only hit. The terminal is short,
    // so the hit starts below the visible text area and the search has to
    // scroll to it -- with the gutter's total row taking a row of that area.
    let mut text = String::new();
    for _ in 0..18 {
        text.push_str("plain\n");
    }
    text.push_str("needle\n");
    let path = scratch_file("search_jump_past_edge.txt");
    std::fs::write(&path, text).expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(8, 200);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("needle");
    kk.send_ctrl('s');

    // The cursor lands on the hit: the status line says so wherever the scroll
    // left it, so the hit line itself must be on screen and carrying the cursor.
    kk.wait_until("the cursor reached the hit", |h| {
        h.screen_contains(":19:1]")
    });
    assert!(
        kk.screen_text().contains("needle"),
        "the cursor's line is on screen:\n{}",
        kk.screen_text()
    );

    // `C-c` is not bound in the prompt, so leave it before asking kk to quit.
    kk.send_ctrl('g');
    kk.wait_until("the prompt is left", |h| !h.screen_contains("Search:"));

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_jump_to_a_hit_at_the_edges_keeps_the_cursor_visible_with_totals() {
    // A hit above the viewport, a run of plain lines the jump lands past, and a
    // hit below -- so the frame shows a top total, a bottom total, and the text
    // between them. The jump must leave the cursor in a row the text actually
    // occupies, not one a summary covers.
    let mut text = String::new();
    text.push_str("needle\n"); // row 1: above the viewport once we scroll
    for _ in 0..20 {
        text.push_str("plain\n");
    }
    text.push_str("needle\n"); // row 22: the line the jump lands on
    for _ in 0..20 {
        text.push_str("plain\n");
    }
    text.push_str("needle\n"); // row 43: below the viewport
    let path = scratch_file("search_jump_totals.txt");
    std::fs::write(&path, text).expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(8, 200);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("needle");
    // `C-s` steps to the next hit each time: row 1, then row 22, then row 43.
    kk.send_ctrl('s');
    kk.wait_until("on the first hit", |h| h.screen_contains(":1:1]"));
    kk.send_ctrl('s');
    kk.wait_until("on the middle hit", |h| h.screen_contains(":22:1]"));

    // The cursor is on row 22 and both totals are on screen; the hit line must
    // still be painted, which is what a scroll against the full height would
    // lose once a summary covers the row the cursor was placed on.
    assert!(
        kk.screen_text().contains("| needle"),
        "the cursor's line is drawn as a buffer line while both totals are shown:\n{}",
        kk.screen_text()
    );
    assert!(
        kk.screen_text().contains(": ") || kk.screen_text().contains(":\n"),
        "a total row is shown, making the height matter:\n{}",
        kk.screen_text()
    );

    kk.send_ctrl('g');
    kk.wait_until("the prompt is left", |h| !h.screen_contains("Search:"));

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn the_gutter_summary_row_is_gone_at_the_top_of_the_buffer() {
    let path = scratch_file("search_gutter_top.txt");
    std::fs::write(&path, "needle\nplain\nneedle\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.resize(24, 200);
    kk.wait_for_text("Opened");

    kk.send_ctrl('s');
    kk.wait_until("query prompt", |h| h.screen_contains("Search:"));
    kk.send_text("needle");

    // The viewport is on the buffer's first row, where no hit can lie above it,
    // so no top total is drawn and the first row stays a line: `1 | needle`,
    // not `0 : needle`.
    kk.wait_until("the first row is a line", |h| {
        h.screen_text().contains("1 | needle")
    });
    assert!(
        !kk.screen_text().contains("0 :"),
        "no empty top total is drawn at the first row:\n{}",
        kk.screen_text()
    );

    kk.send_ctrl('g');
    kk.wait_until("the prompt is left", |h| !h.screen_contains("Search:"));

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
