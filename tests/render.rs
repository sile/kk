//! Properties and examples of the renderers.

mod helpers_frame;

use helpers_frame::row_text;

/// A state over `text`, cursored at the start.
fn state_of(text: &str) -> kk::State {
    kk::State::new(kk::TextBuffer::from_text(text))
}

/// A fresh frame of the given size.
fn frame_of(rows: usize, cols: usize) -> tuinix::Frame {
    tuinix::Frame::new(tuinix::Size { rows, cols })
}

/// How many cells of `frame` a renderer actually wrote to.
///
/// `chars()` visits every cell and reports unwritten ones as blank, so a count
/// of painted cells filters those out.
fn painted_cells(frame: &tuinix::Frame) -> usize {
    frame.chars().filter(|(_, ch)| !ch.is_blank()).count()
}

#[test]
fn the_text_area_never_paints_outside_the_frame() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let rows = noprop::sample_usize_in(ctx, 1..=6);
        let cols = noprop::sample_usize_in(ctx, 1..=12);
        let lines = noprop::sample_usize_in(ctx, 1..=5);
        let mut text = String::new();
        for _ in 0..lines {
            let len = noprop::sample_usize_in(ctx, 0..=15);
            text.push_str(&noprop::sample_ascii_printable_string(ctx, len));
            text.push('\n');
        }

        let mut state = state_of(&text);
        state.viewport = kk::TextPosition {
            row: noprop::sample_usize_in(ctx, 0..=4),
            col: noprop::sample_usize_in(ctx, 0..=4),
        };
        state.cursor = state.viewport;
        let size = tuinix::Size { rows, cols };
        let mut frame = frame_of(rows, cols);

        kk::TextAreaRenderer.render(&state, &mut frame);

        for (position, _) in frame.chars() {
            assert!(
                position.row < size.rows && position.col < size.cols,
                "painted {position:?} outside {size:?}"
            );
        }
        Ok(())
    })?;

    Ok(())
}

#[test]
fn the_text_area_paints_the_visible_slice_from_the_viewport() {
    let mut state = state_of("one\ntwo\nthree\n");
    state.viewport = kk::TextPosition { row: 1, col: 1 };
    let mut frame = frame_of(2, 3);

    kk::TextAreaRenderer.render(&state, &mut frame);

    assert_eq!(row_text(&frame, 0, 3), "wo ");
    assert_eq!(row_text(&frame, 1, 3), "hre");
}

#[test]
fn an_empty_buffer_paints_nothing() {
    let state = state_of("");
    let mut frame = frame_of(3, 5);

    kk::TextAreaRenderer.render(&state, &mut frame);

    assert_eq!(painted_cells(&frame), 0, "an empty buffer paints nothing");
}

#[test]
fn the_text_area_stops_at_the_last_line() {
    let state = state_of("one\n");
    let mut frame = frame_of(5, 5);

    kk::TextAreaRenderer.render(&state, &mut frame);

    assert_eq!(row_text(&frame, 0, 5), "one  ");
    assert_eq!(painted_cells(&frame), 3, "the rows below stay unwritten");
}

#[test]
fn the_status_line_reports_path_and_position() {
    let mut state = state_of("one\ntwo\n");
    let mut frame = frame_of(1, 40);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    let text = row_text(&frame, 0, 40);
    assert!(
        text.starts_with(" [test.txt:1:1] "),
        "row 1, column 1: {text:?}"
    );

    state.handle_char_insert('!');
    let mut frame = frame_of(1, 40);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    let text = row_text(&frame, 0, 40);
    assert!(
        text.starts_with(" [test.txt:1:2] "),
        "one column to the right: {text:?}"
    );
}

#[test]
fn the_status_line_counts_the_matches_up_to_the_cursor() {
    let mut state = state_of("alpha\nbeta\ngamma\nbeta\n");
    state.search_mode = Some(kk::SearchMode::new(true));
    // Typing into the query re-runs it and fills in the highlight.
    for ch in "beta".chars() {
        state.handle_char_insert(ch);
    }
    // The cursor is put where each case wants it; the highlight does not move
    // with it.
    state.cursor = kk::TextPosition { row: 0, col: 0 };

    let mut frame = frame_of(1, 40);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    assert!(
        row_text(&frame, 0, 40).contains("0/2"),
        "before the first match: {:?}",
        row_text(&frame, 0, 40)
    );

    state.cursor = kk::TextPosition { row: 1, col: 2 };
    let mut frame = frame_of(1, 40);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    assert!(
        row_text(&frame, 0, 40).contains("1/2"),
        "inside the first match: {:?}",
        row_text(&frame, 0, 40)
    );

    state.cursor = kk::TextPosition { row: 3, col: 0 };
    let mut frame = frame_of(1, 40);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    assert!(
        row_text(&frame, 0, 40).contains("2/2"),
        "at the second match: {:?}",
        row_text(&frame, 0, 40)
    );
}

#[test]
fn the_status_line_shows_zero_matches_while_the_query_is_empty() {
    let mut state = state_of("alpha\n");
    state.search_mode = Some(kk::SearchMode::new(true));
    let mut frame = frame_of(1, 40);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    assert!(
        row_text(&frame, 0, 40).contains("0/0"),
        "an empty query has no matches: {:?}",
        row_text(&frame, 0, 40)
    );
}

#[test]
fn the_status_line_omits_the_match_count_outside_a_search() {
    let state = state_of("alpha\n");
    let mut frame = frame_of(1, 40);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    let text = row_text(&frame, 0, 40);
    assert!(
        text.starts_with(" [test.txt:1:1]  "),
        "no count without a search: {text:?}"
    );
}

#[test]
fn the_status_line_pads_the_whole_row() {
    let state = state_of("x\n");
    let mut frame = frame_of(1, 60);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    assert_eq!(
        painted_cells(&frame),
        60,
        "the reverse-video bar reaches the right edge"
    );
}

#[test]
fn the_status_line_shows_only_the_clipboards_first_line() {
    let mut state = state_of("one\n");
    state.clipboard.write("copied\nsecond line");
    let mut frame = frame_of(1, 60);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    let text = row_text(&frame, 0, 60);
    assert!(text.contains("copied"), "the first line shows: {text:?}");
    assert!(!text.contains("second"), "the rest does not: {text:?}");
}

#[test]
fn the_message_line_paints_the_message_at_the_origin() {
    let mut state = state_of("one\n");
    state.set_message("Saved 4 chars");
    let mut frame = frame_of(1, 20);

    kk::MessageLineRenderer.render(&state, &mut frame);

    assert_eq!(row_text(&frame, 0, 20), "Saved 4 chars       ");
}

#[test]
fn the_message_line_leaves_a_frame_without_a_message_untouched() {
    let state = state_of("one\n");
    let mut frame = frame_of(2, 10);

    kk::MessageLineRenderer.render(&state, &mut frame);

    assert_eq!(
        painted_cells(&frame),
        0,
        "nothing is written without a message"
    );
}

#[test]
fn the_message_line_prompts_and_shows_the_query_while_a_search_is_open() {
    let mut state = state_of("one\n");
    state.search_mode = Some(kk::SearchMode::new(true));
    for ch in "ab".chars() {
        state.handle_char_insert(ch);
    }
    let mut frame = frame_of(1, 20);

    kk::MessageLineRenderer.render(&state, &mut frame);

    assert_eq!(row_text(&frame, 0, 20), "Search: ab          ");
}

#[test]
fn the_query_hides_the_pending_message() {
    let mut state = state_of("one\n");
    state.set_message("Entered search mode");
    state.search_mode = Some(kk::SearchMode::new(true));
    let mut frame = frame_of(1, 20);

    kk::MessageLineRenderer.render(&state, &mut frame);

    assert_eq!(
        row_text(&frame, 0, 20),
        "Search:             ",
        "the prompt wins over the message"
    );
}

#[test]
fn the_query_cursor_follows_the_prompt_and_the_typed_query() {
    let mut state = state_of("one\n");
    state.search_mode = Some(kk::SearchMode::new(true));
    let region = tuinix::Region {
        position: tuinix::Position { row: 3, col: 0 },
        size: tuinix::Size { rows: 1, cols: 20 },
    };

    let search = state.search_mode.as_ref().expect("search mode");
    assert_eq!(search.cursor_position(region).row, 3);
    assert_eq!(
        search.cursor_position(region).col,
        kk::str_cols("Search: "),
        "an empty query leaves the cursor after the prompt"
    );

    for ch in "ab".chars() {
        state.handle_char_insert(ch);
    }
    let search = state.search_mode.as_ref().expect("search mode");
    assert_eq!(
        search.cursor_position(region).col,
        kk::str_cols("Search: ab")
    );
}
