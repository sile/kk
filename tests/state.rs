//! Properties and examples of the editor state's handlers.

use std::cell::Cell;

/// A state over `text`. Nothing touches the file system.
fn state_of(text: &str) -> kk::State {
    kk::State::new(kk::TextBuffer::from_text(text))
}

/// The buffer's text as the edge would write it.
fn saved_text(state: &kk::State) -> String {
    state.buffer.to_text()
}

/// A cursor at `(row, col)`.
fn at(row: usize, col: usize) -> kk::TextPosition {
    kk::TextPosition { row, col }
}

/// A text area of the given size, for the handlers that need one.
fn area(rows: usize, cols: usize) -> tuinix::Size {
    tuinix::Size { rows, cols }
}

/// One edit the undo property applies to a state.
#[derive(Debug, Clone, Copy)]
enum Edit {
    /// Inserts a character at the cursor.
    Insert(char),

    /// Splits the line at the cursor.
    Newline,

    /// Deletes the character under the cursor.
    Delete,

    /// Kills from the cursor to the end of the line.
    KillLine,
}

impl Edit {
    fn apply(self, state: &mut kk::State) {
        // Close any previous run first, so this edit gets its own snapshot and
        // the undo below drops exactly it.
        state.finish_editing();
        match self {
            Edit::Insert(ch) => {
                state.handle_char_insert(ch);
            }
            Edit::Newline => state.handle_newline_insert(),
            Edit::Delete => state.handle_char_delete_forward(),
            Edit::KillLine => state.handle_line_delete(),
        }
    }
}

#[test]
fn undo_restores_the_text_from_before_the_edit_run() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);
    let restored = Cell::new(0usize);

    runner.run(256, |ctx| {
        let mut state = state_of("alpha\nbeta\ngamma\n");
        let edits = noprop::sample_usize_in(ctx, 1..=4);

        for _ in 0..edits {
            // Each edit is its own run: `finish_editing` between them makes one
            // snapshot per edit, so one undo drops exactly one edit.
            state.cursor = at(noprop::sample_usize_in(ctx, 0..state.buffer.rows()), 0);
            let before = saved_text(&state);
            let edit = noprop::sample_weighted_index(ctx, &[4, 2, 2, 1]);
            match edit {
                0 => Edit::Insert(noprop::sample_ascii_printable_char(ctx)).apply(&mut state),
                1 => Edit::Newline.apply(&mut state),
                2 => Edit::Delete.apply(&mut state),
                _ => Edit::KillLine.apply(&mut state),
            }

            if saved_text(&state) == before {
                // The edit was a no-op (deleting past the end, killing an
                // already-empty tail), so there is nothing to undo.
                continue;
            }

            state.handle_buffer_undo();
            assert_eq!(
                saved_text(&state),
                before,
                "undo did not restore {before:?}"
            );
            restored.set(restored.get() + 1);
        }
        Ok(())
    })?;

    assert!(
        restored.get() > 0,
        "no case performed an undoable edit\n{runner}"
    );
    Ok(())
}

#[test]
fn editing_keeps_the_cursor_inside_the_viewport() {
    let mut state = state_of("one\ntwo\nthree\n");
    let size = area(2, 4);

    state.cursor = at(2, 3);
    state.adjust_viewport(size);

    assert!(state.viewport.row <= state.cursor.row);
    assert!(state.cursor.row < state.viewport.row + size.rows);
    assert!(state.viewport.col <= state.cursor.col);
    assert!(state.cursor.col < state.viewport.col + size.cols);
}

#[test]
fn a_char_insert_reports_and_advances_by_the_character_width() {
    let mut state = state_of("ab\n");
    state.cursor = at(0, 1);

    state.handle_char_insert('X');
    assert_eq!(state.cursor, at(0, 2));
    assert_eq!(saved_text(&state), "aXb\n");

    state.handle_char_insert('一');
    assert_eq!(state.cursor, at(0, 4), "a wide character is two columns");
    assert_eq!(saved_text(&state), "aX一b\n");
}

#[test]
fn line_delete_kills_to_the_end_of_the_line_into_the_clipboard() {
    let mut state = state_of("hello world\nnext\n");
    state.cursor = at(0, 5);

    state.handle_line_delete();

    assert_eq!(saved_text(&state), "hello\nnext\n");
    assert_eq!(state.clipboard.read(), " world");
    assert_eq!(state.clipboard.summary_line, " world");
    assert_eq!(state.cursor, at(0, 5));
}

#[test]
fn line_delete_at_the_end_of_a_line_kills_the_newline() {
    let mut state = state_of("one\ntwo\n");
    state.cursor = at(0, 3);

    state.handle_line_delete();

    assert_eq!(saved_text(&state), "onetwo\n");
    assert_eq!(state.clipboard.read(), "\n");
}

#[test]
fn mark_cut_deletes_the_region_and_leaves_the_cursor_at_its_start() {
    let mut state = state_of("hello world\n");
    state.cursor = at(0, 6);
    state.handle_mark_set();
    state.cursor = at(0, 11);

    state.handle_mark_cut();

    assert_eq!(state.clipboard.read(), "world");
    assert_eq!(saved_text(&state), "hello \n");
    assert_eq!(state.cursor, at(0, 6));
}

#[test]
fn pasting_clipboard_text_inserts_it_at_the_cursor() {
    let mut state = state_of("one\n");
    state.clipboard.write("a\nb");
    state.cursor = at(0, 3);

    state.handle_clipboard_paste();

    assert_eq!(saved_text(&state), "onea\nb\n");
    assert_eq!(state.cursor, at(1, 1), "past the pasted text");
}

#[test]
fn pasting_an_empty_clipboard_changes_nothing() {
    let mut state = state_of("one\n");

    state.handle_clipboard_paste();

    assert_eq!(saved_text(&state), "one\n");
    assert_eq!(
        state.message.as_deref(),
        Some("Clipboard is empty"),
        "and it says so"
    );
}

#[test]
fn saving_and_reloading_round_trip_through_the_edge() {
    let mut state = state_of("one\n");
    state.cursor = at(0, 1);
    state.handle_char_insert('X');

    // Saving is a handshake: the core renders the text, the edge writes it,
    // then the core is told how many characters went out.
    let text = state.handle_buffer_save();
    assert_eq!(text, "oXne\n");
    assert_eq!(state.message.as_deref(), Some("Saving"));

    state.report_saved(text.chars().count());
    assert_eq!(state.message.as_deref(), Some("Saved 5 chars"));

    // Reloading receives the text the edge read back. The cursor was on row 0,
    // which still exists, so only the column is clamped to the new line's width.
    state.handle_buffer_reload("fresh\n");
    assert_eq!(saved_text(&state), "fresh\n");
    assert_eq!(state.cursor, at(0, 2), "column 2 fits in a 5-column line");

    // A cursor on the row one past the last line is left there with column 0,
    // because the clamp compares against `rows()` rather than the last index.
    state.cursor = at(1, 0);
    state.handle_buffer_reload("fresh\n");
    assert_eq!(state.cursor, at(1, 0));
}

#[test]
fn reloading_a_longer_buffer_keeps_the_column() {
    let mut state = state_of("one\ntwo\nthree\n");
    state.cursor = at(1, 1);

    state.handle_buffer_reload("a\nbbbb\nc\nd\n");

    assert_eq!(state.cursor, at(1, 1), "row 1 still exists and has room");
}

#[test]
fn reloading_clamps_a_column_past_the_new_line_end() {
    let mut state = state_of("one\ntwo\nthree\n");
    state.cursor = at(2, 5);

    state.handle_buffer_reload("a\nb\ncd\n");

    assert_eq!(
        state.cursor,
        at(2, 2),
        "column 5 does not fit in a 2-column line"
    );
}

#[test]
fn undo_reports_when_there_is_nothing_left() {
    let mut state = state_of("one\n");

    state.handle_buffer_undo();

    assert_eq!(state.message.as_deref(), Some("Nothing to undo"));
}

#[test]
fn undo_works_without_a_break_between_the_edit_and_the_undo() {
    // A real session does not close the edit run before the user presses
    // `C-u`, so an undo that is the very next thing after an insert must drop
    // that insert, not silently do nothing.
    let mut state = state_of("one\n");

    state.handle_char_insert('x');
    state.handle_buffer_undo();

    assert_eq!(saved_text(&state), "one\n", "the insert was not undone");
}

#[test]
fn a_run_of_inserts_is_one_snapshot_then_there_is_nothing_left() {
    let mut state = state_of("one\n");

    // A run of inserts is one snapshot, so the first undo drops all of it and
    // the second reports there is nothing older to restore.
    state.handle_char_insert('a');
    state.handle_char_insert('b');
    state.finish_editing();

    state.handle_buffer_undo();
    assert_eq!(saved_text(&state), "one\n");

    state.handle_buffer_undo();
    assert_eq!(state.message.as_deref(), Some("Nothing to undo"));
}

#[test]
fn a_recenter_request_centres_the_cursor_and_is_consumed() {
    let mut state = state_of("one\ntwo\nthree\n");
    state.cursor = at(2, 0);
    state.handle_view_recenter();

    state.adjust_viewport(area(10, 10));

    assert_eq!(
        state.viewport.row, 0,
        "2 is centered in a 10-row area at row 0"
    );
    assert!(!state.recenter_viewport, "the request is used once");
}

#[test]
fn a_click_moves_the_cursor_through_the_viewport() {
    let mut state = state_of("one\ntwo\nthree\nfour\n");
    state.viewport = at(2, 1);

    // Row 1 of the visible area is buffer row 3; column 1 is buffer column 2.
    state.handle_cursor_to_screen_position(1, 1);

    assert_eq!(state.cursor, at(3, 2));
}

#[test]
fn a_click_below_the_buffer_lands_on_the_last_line() {
    let mut state = state_of("one\ntwo\n");

    state.handle_cursor_to_screen_position(20, 0);

    assert_eq!(
        state.cursor,
        at(2, 0),
        "row 2 is the row after the last line"
    );
}

#[test]
fn a_click_past_the_line_end_lands_on_the_line_end() {
    let mut state = state_of("one\ntwo\nthree\n");

    state.handle_cursor_to_screen_position(1, 40);

    assert_eq!(state.cursor, at(1, 3), "'two' is 3 columns wide");
}

#[test]
fn a_click_snaps_back_onto_a_character_boundary() {
    let mut state = state_of("aあ\n");

    // 'あ' is two columns wide and starts at column 1, so column 2 is inside it.
    state.handle_cursor_to_screen_position(0, 2);

    assert_eq!(
        state.cursor,
        at(0, 1),
        "the cursor sits on 'あ', not inside it"
    );
}

#[test]
fn the_start_position_moves_the_cursor_absolutely() {
    let mut state = state_of("one\ntwo\nthree\n");

    // The handler takes 0-based positions, so row 1 is the second line.
    state.handle_cursor_to_position(1, 2);

    assert_eq!(state.cursor, at(1, 2));
}

#[test]
fn the_start_position_is_clamped_to_the_buffer() {
    let mut state = state_of("one\ntwo\n");

    state.handle_cursor_to_position(99, 99);

    assert_eq!(
        state.cursor,
        at(2, 0),
        "row 2 is the row after the last line, and it has no columns"
    );
}

#[test]
fn the_start_position_is_clamped_to_the_line_end() {
    let mut state = state_of("one\ntwo\nthree\n");

    state.handle_cursor_to_position(1, 40);

    assert_eq!(state.cursor, at(1, 3), "'two' is 3 columns wide");
}

#[test]
fn the_start_position_snaps_back_onto_a_character_boundary() {
    let mut state = state_of("aあ\n");

    // 'あ' is two columns wide and starts at column 1, so column 2 is inside it.
    state.handle_cursor_to_position(0, 2);

    assert_eq!(
        state.cursor,
        at(0, 1),
        "the cursor sits on 'あ', not inside it"
    );
}

#[test]
fn the_start_position_is_not_a_relative_move() {
    let mut state = state_of("one\ntwo\nthree\nfour\n");
    state.viewport = at(2, 1);

    state.handle_cursor_to_position(0, 0);

    assert_eq!(
        state.cursor,
        at(0, 0),
        "the viewport is only added by the screen-relative handler"
    );
}

#[test]
fn scrolling_down_moves_the_cursor_and_the_viewport_together() {
    let mut state = state_of("a\nb\nc\nd\ne\nf\ng\n");

    state.handle_scroll(3);

    assert_eq!(state.cursor.row, 3, "three rows down from row 0");
    state.adjust_viewport(area(3, 10));
    assert_eq!(
        state.viewport.row, 1,
        "the viewport follows so the cursor stays visible"
    );
}

#[test]
fn scrolling_up_stops_at_the_first_line() {
    let mut state = state_of("a\nb\n");

    state.handle_scroll(-5);

    assert_eq!(state.cursor.row, 0);
}

#[test]
fn scrolling_down_stops_at_the_row_after_the_last_line() {
    let mut state = state_of("a\nb\n");

    state.handle_scroll(50);

    assert_eq!(
        state.cursor.row, 2,
        "the clamp is `rows()`, not the last index"
    );
}
