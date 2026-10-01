//! Properties and examples of the in-buffer search.

use std::cell::Cell;

use kk::{Highlight, SearchPrompt};

/// A state over `text`, with no search open yet.
fn state_of(text: &str) -> kk::State {
    kk::State::new(kk::TextBuffer::new(text))
}

/// Runs `query` over `state` and returns the matches.
fn run(query: &str, state: &kk::State) -> Highlight {
    let mut search = SearchPrompt::new();
    for ch in query.chars() {
        search.insert_char(ch);
    }
    search.search(&state.buffer)
}

/// The columns of the match starts in row 0.
fn hit_cols(highlight: &Highlight) -> Vec<usize> {
    highlight
        .items()
        .iter()
        .map(|item| item.start_position.col)
        .collect()
}

#[test]
fn a_search_finds_it_again_in_each_run() {
    for needle in ["x", "xx"] {
        let state = state_of("xxx\n");
        let highlight = run(needle, &state);

        assert_eq!(
            highlight.len(),
            3 - (needle.len() - 1),
            "overlapping matches for {needle:?}"
        );
    }
}

#[test]
fn a_search_reports_where_each_match_starts_and_ends() {
    let state = state_of("one two one\n");

    let highlight = run("one", &state);

    assert_eq!(hit_cols(&highlight), vec![0, 8]);
    assert_eq!(
        highlight.items()[0].start_position,
        kk::TextPosition { row: 0, col: 0 }
    );
    assert_eq!(
        highlight.items()[0].end_position,
        kk::TextPosition { row: 0, col: 3 }
    );
}

#[test]
fn a_search_counts_matches_across_lines() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let found = Cell::new(0usize);
    let absent = Cell::new(0usize);
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        // Build every line out of the same token, so the count of matches is
        // known without modelling the search: with no self-overlap, a line that
        // holds the token `n` times has exactly `n` matches.
        let rows = noprop::sample_usize_in(ctx, 1..=3);
        let per_row = noprop::sample_usize_in(ctx, 0..=3);
        let token = noprop::sample_choice(ctx, &["ab", "cd"]);
        let filler = noprop::sample_choice(ctx, &["-", "--", " "]);
        let mut text = String::new();
        for _ in 0..rows {
            for _ in 0..per_row {
                text.push_str(token);
                text.push_str(filler);
            }
            text.push('\n');
        }
        let state = state_of(&text);

        let highlight = run(token, &state);

        assert_eq!(
            highlight.len(),
            rows * per_row,
            "matches for {token:?} in {text:?}"
        );
        for item in highlight.items() {
            assert_eq!(item.end_position.row, item.start_position.row);
            assert_eq!(
                item.end_position.col - item.start_position.col,
                token.len(),
                "match width in {text:?}"
            );
            assert!(
                highlight.contains(item.start_position),
                "a match contains its own start"
            );
            assert!(
                !highlight.contains(item.end_position),
                "a match does not contain its exclusive end"
            );
        }

        if highlight.is_empty() {
            absent.set(absent.get() + 1);
        } else {
            found.set(found.get() + 1);
        }
        Ok(())
    })?;

    assert!(found.get() > 0, "no case produced a match\n{runner}");
    assert!(
        absent.get() > 0,
        "no case produced an empty result\n{runner}"
    );
    Ok(())
}

#[test]
fn a_count_of_the_matches_up_to_a_position_is_the_matches_reached() {
    let state = state_of("one two one\n");
    let highlight = run("one", &state);
    let at = |row, col| kk::TextPosition { row, col };

    assert_eq!(
        highlight.count_up_to(at(0, 0)),
        1,
        "the first match's start"
    );
    assert_eq!(highlight.count_up_to(at(0, 1)), 1, "inside the first match");
    assert_eq!(highlight.count_up_to(at(0, 3)), 1, "just past the first");
    assert_eq!(
        highlight.count_up_to(at(0, 8)),
        2,
        "the second match's start"
    );
    assert_eq!(highlight.count_up_to(at(0, 11)), 2, "at the end");
}

#[test]
fn a_count_of_the_matches_up_to_a_position_is_zero_before_the_first() {
    let state = state_of("one two one\n");
    let highlight = run("two", &state);

    assert_eq!(
        highlight.count_up_to(kk::TextPosition { row: 0, col: 3 }),
        0,
        "the cursor has not reached the match yet"
    );
    assert_eq!(
        highlight.count_up_to(kk::TextPosition { row: 0, col: 4 }),
        1,
        "the match starts here"
    );
}

#[test]
fn a_search_is_case_insensitive() {
    let state = state_of("Alpha BETA\n");

    assert_eq!(run("alpha", &state).len(), 1);
    assert_eq!(run("beta", &state).len(), 1);
    assert_eq!(run("ALPHA", &state).len(), 1);
}

#[test]
fn an_empty_query_matches_nothing() {
    let state = state_of("abc\n");

    let highlight = run("", &state);

    assert!(highlight.is_empty());
    assert!(!highlight.contains(kk::TextPosition { row: 0, col: 0 }));
}

#[test]
fn a_query_longer_than_the_line_matches_nothing() {
    let state = state_of("ab\n");

    assert!(run("abc", &state).is_empty());
}

#[test]
fn a_row_count_is_the_matches_that_start_on_it() {
    // Rows 0 and 2 hold hits, row 1 does not, so the count must tell the two
    // kinds apart rather than answer "some" or "none".
    let state = state_of("one one\nplain\none\n");
    let highlight = run("one", &state);

    assert_eq!(highlight.count_on_row(0), 2);
    assert_eq!(highlight.count_on_row(1), 0);
    assert_eq!(highlight.count_on_row(2), 1);
    // Past the last row that holds a hit, which the binary search brackets too.
    assert_eq!(highlight.count_on_row(3), 0);
}

#[test]
fn a_row_count_is_zero_with_no_matches_at_all() {
    let state = state_of("abc\n");

    let highlight = run("z", &state);

    assert!(highlight.is_empty());
    assert_eq!(highlight.count_on_row(0), 0);
}

#[test]
fn the_totals_outside_a_slice_split_the_hits_at_its_edges() {
    // Row 0 holds two hits, rows 1 and 2 hold none, row 3 holds one.
    let state = state_of("x x\nplain\nplain\nx\n");
    let highlight = run("x", &state);

    // A slice over the blank rows has one hit above it and one below.
    assert_eq!(highlight.count_outside(1, 3), (2, 1));
    // A slice over everything has nothing outside it.
    assert_eq!(highlight.count_outside(0, 4), (0, 0));
    // A slice at the very start keeps every hit below it.
    assert_eq!(highlight.count_outside(0, 0), (0, 3));
    // A slice past the end keeps every hit above it.
    assert_eq!(highlight.count_outside(4, 9), (3, 0));
}

#[test]
fn the_totals_outside_a_slice_are_zero_with_no_matches_at_all() {
    let state = state_of("abc\ndef\n");

    let highlight = run("z", &state);

    assert_eq!(highlight.count_outside(0, 1), (0, 0));
    assert_eq!(highlight.count_outside(1, 2), (0, 0));
}

#[test]
fn the_query_cursor_is_edited_independently_of_the_buffer_cursor() {
    let mut state = state_of("abc\n");
    state.search_prompt = Some(SearchPrompt::new());
    let buffer_cursor = state.cursor;

    for ch in "xy".chars() {
        state.handle_char_insert(ch);
    }
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .query(),
        "xy"
    );
    assert_eq!(
        state.cursor, buffer_cursor,
        "the buffer cursor is untouched"
    );

    // The movement handlers drive the query cursor while a search is open.
    state.handle_cursor_left();
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .cursor(),
        1
    );
    state.handle_char_delete_backward();
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .query(),
        "y"
    );
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .cursor(),
        0
    );
    state.handle_cursor_line_end();
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .cursor(),
        1
    );
    state.handle_cursor_line_start();
    assert_eq!(
        state
            .search_prompt
            .as_ref()
            .expect("the prompt is open")
            .cursor(),
        0
    );

    // The query re-runs as it is edited, so the highlight tracks it.
    assert_eq!(state.highlight.len(), 0, "'y' is not in the buffer");
    assert_eq!(state.cursor, buffer_cursor);
}

#[test]
fn typing_a_query_fills_in_the_highlight() {
    let mut state = state_of("abc abc\n");
    state.search_prompt = Some(SearchPrompt::new());

    for ch in "abc".chars() {
        state.handle_char_insert(ch);
    }

    assert_eq!(state.highlight.len(), 2);
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 0 });
}

#[test]
fn typing_a_query_leaves_the_cursor_alone_even_when_it_matches() {
    let mut state = state_of("one two one\n");
    state.search_prompt = Some(SearchPrompt::new());
    state.cursor = kk::TextPosition { row: 0, col: 4 };

    // Every character matches, so the old behaviour would have jumped to the
    // first hit on the first keypress.
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }

    assert_eq!(state.highlight.len(), 2, "the matches are found");
    assert_eq!(
        state.cursor,
        kk::TextPosition { row: 0, col: 4 },
        "the cursor stays put until a hit is asked for"
    );

    // `C-s` is what moves it, and it jumps past the cursor to the hit after it.
    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });
}

#[test]
fn the_next_hit_advances_and_wraps_around() {
    let mut state = state_of("one two one\n");
    state.search_prompt = Some(SearchPrompt::new());
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }

    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });

    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 0 }, "wraps");
}

#[test]
fn a_hit_step_leaves_a_pending_recenter_request_alone() {
    let mut state = state_of("one two one\n");
    state.search_prompt = Some(SearchPrompt::new());
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }

    // A recenter some earlier command asked for is still pending when the
    // step runs, because the render path has not consumed it yet. The step
    // must not cancel it: the request is about the cursor, not the search.
    state.handle_view_recenter();
    assert!(state.recenter_viewport.is_some());

    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });
    assert!(
        state.recenter_viewport.is_some(),
        "the step carries the request through instead of clearing it"
    );
}

/// A buffer with a hit on every row, so a step always lands on a known row.
fn state_of_hits(rows: usize) -> kk::State {
    let mut text = String::new();
    for _ in 0..rows {
        text.push_str("hit x\n");
    }
    let mut state = state_of(&text);
    state.search_prompt = Some(SearchPrompt::new());
    for ch in "hit".chars() {
        state.handle_char_insert(ch);
    }
    state
}

#[test]
fn a_hit_already_on_screen_leaves_the_viewport_alone() {
    let mut state = state_of_hits(10);
    // The cursor starts on row 0 and the text area shows rows 0..3.
    let text_area = tuinix::Size { rows: 3, cols: 80 };
    state.adjust_viewport(text_area);
    assert_eq!(state.viewport.row, 0, "the first row is at the top");

    // The next hit, on row 1, is already visible.
    state.handle_search_next_hit();
    state.adjust_viewport(text_area);

    assert_eq!(state.cursor.row, 1, "the cursor moved to the hit");
    assert_eq!(state.viewport.row, 0, "the viewport did not move");
}

#[test]
fn a_hit_below_the_text_area_scrolls_just_far_enough() {
    let mut state = state_of_hits(10);
    let text_area = tuinix::Size { rows: 3, cols: 80 };
    state.adjust_viewport(text_area);

    // Walk to the hit on row 4, past the last visible row (2).
    for _ in 0..4 {
        state.handle_search_next_hit();
        state.adjust_viewport(text_area);
    }

    assert_eq!(state.cursor.row, 4, "the cursor moved to the hit");
    // The prompt is open, so the gutter's two summary rows take two of the
    // three rows the text area has, leaving the hit alone in the one left. The
    // scroll is just enough for that: the viewport sits on the hit's row, the
    // row the summary above leaves blank, not the row it would need with no
    // gutter.
    assert_eq!(
        state.viewport.row, 4,
        "the hit is the one row the summaries leave"
    );

    // The point of the fix: wherever the scroll settles, the cursor's row is
    // one the text is drawn in, so it cannot land on a summary's row.
    let start = state.viewport.row;
    let end = start + state.text_rows(text_area.rows);
    assert!(
        state.cursor.row >= start && state.cursor.row < end,
        "the cursor's row {} is inside the drawn text rows {start}..{end}",
        state.cursor.row
    );
}

#[test]
fn a_step_to_a_hit_a_screen_away_centers_it() {
    // Two hits far apart: one on row 0 and one on row 30, in a 12-row area.
    // Stepping to the row 30 hit leaves most of a screen behind the last
    // visible row, so it no longer shares the screen it left and is centered.
    let mut text = String::new();
    for row in 0..40 {
        if row == 0 || row == 30 {
            text.push_str("hit\n");
        } else {
            text.push_str("x\n");
        }
    }
    let mut state = state_of(&text);
    state.search_prompt = Some(SearchPrompt::new());
    for ch in "hit".chars() {
        state.handle_char_insert(ch);
    }

    let text_area = tuinix::Size { rows: 12, cols: 80 };
    state.adjust_viewport(text_area);

    state.handle_search_next_hit();
    state.adjust_viewport(text_area);

    assert_eq!(state.cursor.row, 30, "the cursor moved to the far hit");
    let rows = state.text_rows(text_area.rows);
    assert_eq!(
        state.viewport.row,
        30 - rows / 2,
        "the hit sits in the middle of the drawn rows"
    );
}

#[test]
fn a_cursor_near_the_right_edge_scrolls_against_the_width_the_gutter_leaves() {
    let mut state = state_of("x = this line is long enough to need a scroll\n");
    state.search_prompt = Some(SearchPrompt::new());
    state.handle_char_insert('x');

    // A column the area's full width reaches but the gutter's five columns do
    // not: without the fix the viewport stays put and the cursor is drawn off
    // the right edge.
    state.cursor = kk::TextPosition { row: 0, col: 25 };
    state.recenter_viewport = None;
    let text_area = tuinix::Size { rows: 3, cols: 30 };
    state.adjust_viewport(text_area);

    assert!(state.text_cols(text_area.cols) < text_area.cols);
    assert_eq!(
        state.viewport.col,
        25 - (state.text_cols(text_area.cols) - 1),
        "the scroll leaves the cursor in the columns the text is drawn in"
    );
    assert!(
        state.cursor.col >= state.viewport.col
            && state.cursor.col < state.viewport.col + state.text_cols(text_area.cols),
        "the cursor's column {} is inside the drawn text columns",
        state.cursor.col
    );
}

#[test]
fn a_hit_above_the_text_area_keeps_the_cursor_in_the_drawn_rows() {
    let mut state = state_of_hits(10);
    let text_area = tuinix::Size { rows: 3, cols: 80 };
    state.adjust_viewport(text_area);

    // Walk down to row 6, then back up to row 2: the scroll follows, with the
    // top summary appearing above the viewport.
    for _ in 0..6 {
        state.handle_search_next_hit();
        state.adjust_viewport(text_area);
    }
    for _ in 0..4 {
        state.handle_search_prev_hit();
        state.adjust_viewport(text_area);
    }

    assert_eq!(
        state.cursor.row, 2,
        "the cursor went back to the earlier hit"
    );
    let start = state.viewport.row;
    let end = start + state.text_rows(text_area.rows);
    assert!(
        state.cursor.row >= start && state.cursor.row < end,
        "the cursor's row {} is inside the drawn text rows {start}..{end}",
        state.cursor.row
    );
}

#[test]
fn the_previous_hit_goes_back_and_wraps_around() {
    let mut state = state_of("one two one\n");
    state.search_prompt = Some(SearchPrompt::new());
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }
    state.cursor = kk::TextPosition { row: 0, col: 10 };

    state.handle_search_prev_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });

    state.handle_search_prev_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 0 });
    state.handle_search_prev_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 }, "wraps");
}

#[test]
fn the_hit_handlers_do_nothing_without_a_search() {
    let mut state = state_of("one two one\n");
    state.cursor = kk::TextPosition { row: 0, col: 4 };

    state.handle_search_next_hit();
    state.handle_search_prev_hit();

    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 4 });
}

#[test]
fn cancelling_a_search_puts_the_cursor_and_viewport_back() {
    let mut state = state_of("one two one\n");
    state.cursor = kk::TextPosition { row: 0, col: 4 };
    state.viewport = kk::TextPosition { row: 0, col: 2 };

    state.handle_search_enter();
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }
    state.handle_search_next_hit();
    state.viewport = kk::TextPosition { row: 0, col: 8 };
    assert_ne!(state.cursor, kk::TextPosition { row: 0, col: 4 });

    state.handle_search_cancel();

    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 4 });
    assert_eq!(state.viewport, kk::TextPosition { row: 0, col: 2 });
    assert!(state.search_prompt.is_none(), "the prompt is gone");
    assert!(state.highlight.is_empty(), "the highlight is gone");
}

#[test]
fn accepting_a_search_keeps_the_cursor_on_the_hit() {
    let mut state = state_of("one two one\n");
    state.cursor = kk::TextPosition { row: 0, col: 4 };

    state.handle_search_enter();
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }
    state.handle_search_next_hit();

    state.handle_search_finish();

    assert_eq!(
        state.cursor,
        kk::TextPosition { row: 0, col: 8 },
        "the hit is where editing resumes"
    );
    assert!(state.search_prompt.is_none(), "the prompt is gone");
    assert!(state.highlight.is_empty(), "the highlight is gone");
}

#[test]
fn a_second_search_starts_from_where_the_last_one_left_the_cursor() {
    let mut state = state_of("one two one\n");
    state.handle_search_enter();
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }
    state.handle_search_next_hit();
    state.handle_search_finish();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });

    // The next prompt remembers the accepted position, not the first one, so
    // cancelling it returns to where the previous search ended.
    state.handle_search_enter();
    state.handle_search_cancel();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });
}

#[test]
fn leaving_a_search_that_was_never_opened_does_nothing() {
    let mut state = state_of("one two one\n");
    state.cursor = kk::TextPosition { row: 0, col: 4 };

    state.handle_search_cancel();
    state.handle_search_finish();

    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 4 });
    assert!(state.search_prompt.is_none());
}
