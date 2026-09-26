//! Properties and examples of the in-buffer search.

use std::cell::Cell;

use kk::{Highlight, SearchMode};

/// A state over `text`, with no search open yet.
fn state_of(text: &str) -> kk::State {
    kk::State::new(kk::TextBuffer::from_text(text))
}

/// Runs `query` over `state` and returns the matches.
fn run(query: &str, state: &kk::State) -> Highlight {
    let mut search = SearchMode::new(true);
    for ch in query.chars() {
        search.insert_char(ch);
    }
    search.search(&state.buffer)
}

/// The columns of the match starts in row 0.
fn hit_cols(highlight: &Highlight) -> Vec<usize> {
    highlight
        .items
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
            highlight.items.len(),
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
        highlight.items[0].start_position,
        kk::TextPosition { row: 0, col: 0 }
    );
    assert_eq!(
        highlight.items[0].end_position,
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
            highlight.items.len(),
            rows * per_row,
            "matches for {token:?} in {text:?}"
        );
        for item in &highlight.items {
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

        if highlight.items.is_empty() {
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

    assert_eq!(run("alpha", &state).items.len(), 1);
    assert_eq!(run("beta", &state).items.len(), 1);
    assert_eq!(run("ALPHA", &state).items.len(), 1);
}

#[test]
fn an_empty_query_matches_nothing() {
    let state = state_of("abc\n");

    let highlight = run("", &state);

    assert!(highlight.items.is_empty());
    assert!(!highlight.contains(kk::TextPosition { row: 0, col: 0 }));
}

#[test]
fn a_query_longer_than_the_line_matches_nothing() {
    let state = state_of("ab\n");

    assert!(run("abc", &state).items.is_empty());
}

#[test]
fn the_query_cursor_is_edited_independently_of_the_buffer_cursor() {
    let mut state = state_of("abc\n");
    state.search_mode = Some(SearchMode::new(true));
    let buffer_cursor = state.cursor;

    for ch in "xy".chars() {
        state.handle_char_insert(ch);
    }
    assert_eq!(
        state.search_mode.as_ref().expect("search mode").query,
        vec!['x', 'y']
    );
    assert_eq!(
        state.cursor, buffer_cursor,
        "the buffer cursor is untouched"
    );

    // The movement handlers drive the query cursor while a search is open.
    state.handle_cursor_left();
    assert_eq!(state.search_mode.as_ref().expect("search mode").cursor, 1);
    state.handle_char_delete_backward();
    assert_eq!(
        state.search_mode.as_ref().expect("search mode").query,
        vec!['y']
    );
    assert_eq!(state.search_mode.as_ref().expect("search mode").cursor, 0);
    state.handle_cursor_line_end();
    assert_eq!(state.search_mode.as_ref().expect("search mode").cursor, 1);
    state.handle_cursor_line_start();
    assert_eq!(state.search_mode.as_ref().expect("search mode").cursor, 0);

    // The query re-runs as it is edited, so the highlight tracks it.
    assert_eq!(state.highlight.items.len(), 0, "'y' is not in the buffer");
    assert_eq!(state.cursor, buffer_cursor);
}

#[test]
fn typing_a_query_fills_in_the_highlight() {
    let mut state = state_of("abc abc\n");
    state.search_mode = Some(SearchMode::new(true));

    for ch in "abc".chars() {
        state.handle_char_insert(ch);
    }

    assert_eq!(state.highlight.items.len(), 2);
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 0 });
}

#[test]
fn typing_a_query_leaves_the_cursor_alone_even_when_it_matches() {
    let mut state = state_of("one two one\n");
    state.search_mode = Some(SearchMode::new(true));
    state.cursor = kk::TextPosition { row: 0, col: 4 };

    // Every character matches, so the old behaviour would have jumped to the
    // first hit on the first keypress.
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }

    assert_eq!(state.highlight.items.len(), 2, "the matches are found");
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
    state.search_mode = Some(SearchMode::new(true));
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }

    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });
    assert!(state.recenter_viewport, "a hit recenters the view");

    state.handle_search_next_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 0 }, "wraps");
}

#[test]
fn the_previous_hit_goes_back_and_wraps_around() {
    let mut state = state_of("one two one\n");
    state.search_mode = Some(SearchMode::new(true));
    for ch in "one".chars() {
        state.handle_char_insert(ch);
    }
    state.cursor = kk::TextPosition { row: 0, col: 10 };

    state.handle_search_prev_hit();
    assert_eq!(state.cursor, kk::TextPosition { row: 0, col: 8 });
    assert!(
        !state.search_mode.as_ref().expect("search mode").forward,
        "the direction flips"
    );

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
