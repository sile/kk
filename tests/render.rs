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

/// The style `frame` painted at `(row, col)`.
///
/// A position no renderer wrote to keeps [`tuinix::Style::RESET`], so an
/// unpainted cell and a painted-but-unstyled one look alike here.
fn style_at(frame: &tuinix::Frame, row: usize, col: usize) -> tuinix::Style {
    let at = tuinix::Position { row, col };
    frame
        .chars()
        .find(|(position, _)| *position == at)
        .map_or(tuinix::Style::RESET, |(_, ch)| ch.style())
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
fn a_search_bolds_and_underlines_the_matches() {
    let mut state = state_of("one two\n");
    state.search_mode = Some(kk::SearchMode::new());
    state.handle_char_insert('t');
    state.handle_char_insert('w');
    state.handle_char_insert('o');
    // The cursor is put away from the match so that no hit column is also the
    // cursor, which would be reversed instead.
    state.cursor = kk::TextPosition { row: 0, col: 0 };
    let mut frame = frame_of(1, 7);

    kk::TextAreaRenderer.render(&state, &mut frame);

    // `two` starts at column 4; its three columns are marked as a hit, `one` is
    // not. A hit is bold and underlined rather than reversed, since the reverse
    // is what the cursor and the mark use.
    for col in 4..7 {
        let style = style_at(&frame, 0, col);
        assert!(style.bold, "column {col} is a match: {style:?}");
        assert!(style.underline, "column {col} is a match: {style:?}");
    }
    let plain = style_at(&frame, 0, 0);
    assert!(!plain.bold && !plain.underline, "`one` is not a match");
}

#[test]
fn a_match_is_not_reversed() {
    let mut state = state_of("one\n");
    state.search_mode = Some(kk::SearchMode::new());
    state.handle_char_insert('o');
    state.handle_char_insert('n');
    state.handle_char_insert('e');
    let mut frame = frame_of(1, 3);

    kk::TextAreaRenderer.render(&state, &mut frame);

    // The cursor sits on the first match, so that one column is reversed; the
    // rest of the match stays bold and underlined.
    assert!(
        !style_at(&frame, 0, 1).reverse,
        "a hit other than the cursor is not reversed"
    );
}

#[test]
fn an_empty_query_leaves_the_text_area_plain_apart_from_the_cursor() {
    let mut state = state_of("one\n");
    state.search_mode = Some(kk::SearchMode::new());
    state.cursor = kk::TextPosition { row: 0, col: 1 };
    let mut frame = frame_of(2, 3);

    kk::TextAreaRenderer.render(&state, &mut frame);

    // The empty query matches nothing, so the text away from the cursor is
    // plainly styled: a search neither dims nor underlines it.
    for col in [0, 2] {
        assert_eq!(
            style_at(&frame, 0, col),
            tuinix::Style::new(),
            "column {col} carries no search style"
        );
    }
}

#[test]
fn a_search_reverses_the_character_under_the_cursor() {
    let mut state = state_of("one\n");
    state.search_mode = Some(kk::SearchMode::new());
    state.cursor = kk::TextPosition { row: 0, col: 1 };
    let mut frame = frame_of(1, 3);

    kk::TextAreaRenderer.render(&state, &mut frame);

    assert!(
        style_at(&frame, 0, 1).reverse,
        "the character under the cursor is reversed"
    );
    assert!(
        !style_at(&frame, 0, 0).reverse,
        "the characters beside it are not"
    );
}

#[test]
fn the_cursor_is_not_reversed_outside_a_search() {
    let mut state = state_of("one\n");
    state.cursor = kk::TextPosition { row: 0, col: 1 };
    let mut frame = frame_of(1, 3);

    kk::TextAreaRenderer.render(&state, &mut frame);

    // The terminal's own cursor marks the position in the buffer, so the cell
    // under it is painted like any other.
    assert!(
        !style_at(&frame, 0, 1).reverse,
        "the cursor carries no style of its own"
    );
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
    state.search_mode = Some(kk::SearchMode::new());
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
    state.search_mode = Some(kk::SearchMode::new());
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

    // A painted cell at the last column means the bar reaches the right edge.
    // The count of painted cells is not the measure here: the clipboard icon is
    // wide, so `chars()` skips its continuation column and the count is short
    // by one even when the row is filled.
    assert_eq!(
        style_at(&frame, 0, 59),
        tuinix::Style::new().reverse().bold(),
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
fn the_status_line_always_shows_the_clipboard_icon() {
    let state = state_of("one\n");
    let mut frame = frame_of(1, 40);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    let text = row_text(&frame, 0, 40);
    assert!(
        text.contains('📋'),
        "an empty clipboard still shows the icon: {text:?}"
    );
}

#[test]
fn the_status_line_puts_the_magnifier_before_the_match_count() {
    let mut state = state_of("alpha\n");
    state.search_mode = Some(kk::SearchMode::new());
    let mut frame = frame_of(1, 40);

    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);

    let text = row_text(&frame, 0, 40);
    assert!(
        text.contains("🔍 0/0"),
        "the icon introduces the count: {text:?}"
    );
}

#[test]
fn the_status_line_shows_the_icon_of_the_context_on_screen() {
    // The prompt has its own clipboard, so the summary shown is the prompt's
    // while it is open and the buffer's otherwise.
    let mut state = state_of("one\n");
    state.clipboard.write("from the buffer");
    state.search_clipboard.write("from the prompt");

    let mut frame = frame_of(1, 60);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    let text = row_text(&frame, 0, 60);
    assert!(
        text.contains("from the buffer"),
        "edit shows its own: {text:?}"
    );
    assert!(
        !text.contains("from the prompt"),
        "and not the other: {text:?}"
    );

    state.search_mode = Some(kk::SearchMode::new());
    let mut frame = frame_of(1, 60);
    kk::StatusLineRenderer.render(&state, "test.txt", &mut frame);
    let text = row_text(&frame, 0, 60);
    assert!(
        text.contains("from the prompt"),
        "search shows its own: {text:?}"
    );
    assert!(
        !text.contains("from the buffer"),
        "and not the other: {text:?}"
    );
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
    state.search_mode = Some(kk::SearchMode::new());
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
    state.search_mode = Some(kk::SearchMode::new());
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
    state.search_mode = Some(kk::SearchMode::new());
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
