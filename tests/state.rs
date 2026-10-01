//! Properties and examples of the editor state's handlers.

use std::cell::Cell;

/// A state over `text`. Nothing touches the file system.
fn state_of(text: &str) -> kk::State {
    kk::State::new(kk::TextBuffer::new(text))
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

    /// Cuts from the cursor to the end of the line.
    CutLineTail,
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
            Edit::CutLineTail => state.handle_line_cut_tail(),
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
                _ => Edit::CutLineTail.apply(&mut state),
            }

            if saved_text(&state) == before {
                // The edit was a no-op (deleting past the end, cutting an
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
fn line_cut_tail_cuts_to_the_end_of_the_line_into_the_clipboard() {
    let mut state = state_of("hello world\nnext\n");
    state.cursor = at(0, 5);

    state.handle_line_cut_tail();

    assert_eq!(saved_text(&state), "hello\nnext\n");
    assert_eq!(state.clipboard.read(), " world");
    assert_eq!(state.clipboard.summary_line(), " world");
    assert_eq!(state.cursor, at(0, 5));
}

#[test]
fn line_cut_tail_at_the_end_of_a_line_cuts_the_newline() {
    let mut state = state_of("one\ntwo\n");
    state.cursor = at(0, 3);

    state.handle_line_cut_tail();

    assert_eq!(saved_text(&state), "onetwo\n");
    assert_eq!(state.clipboard.read(), "\n");
}

#[test]
fn a_run_of_cuts_collects_into_one_clipboard_entry() {
    let mut state = state_of("one\ntwo\nthree\n");
    state.cursor = at(0, 0);

    // Cut `one`, then the newline, then `two` -- one run with no break.
    state.handle_line_cut_tail();
    assert_eq!(saved_text(&state), "\ntwo\nthree\n");
    assert_eq!(state.clipboard.read(), "one");

    state.handle_line_cut_tail();
    assert_eq!(saved_text(&state), "two\nthree\n");
    assert_eq!(state.clipboard.read(), "one\n");

    state.handle_line_cut_tail();
    assert_eq!(saved_text(&state), "\nthree\n");
    assert_eq!(state.clipboard.read(), "one\ntwo");

    assert_eq!(state.clipboard.summary_line(), "one");
}

#[test]
fn a_break_between_cuts_starts_a_new_clipboard_entry() {
    let mut state = state_of("one\ntwo\n");
    state.cursor = at(0, 0);

    state.handle_line_cut_tail();
    assert_eq!(state.clipboard.read(), "one");

    // Moving the cursor is a break, so the next cut replaces the entry.
    state.handle_cursor_down();
    state.handle_line_cut_tail();

    assert_eq!(state.clipboard.read(), "two");
    assert_eq!(state.clipboard.summary_line(), "two");
}

#[test]
fn an_edit_between_cuts_starts_a_new_clipboard_entry() {
    let mut state = state_of("one\ntwo\n");
    state.cursor = at(0, 0);

    state.handle_line_cut_tail();
    state.handle_char_insert('x');
    state.handle_line_cut_tail();

    assert_eq!(state.clipboard.read(), "\n");
}

#[test]
fn a_cut_that_removed_nothing_still_leaves_the_previous_entry_alone() {
    let mut state = state_of("one\n");
    state.cursor = at(0, 3);

    // The only line has no newline to cut, so this cut is a no-op.
    state.handle_line_cut_tail();
    assert_eq!(saved_text(&state), "one\n");
    assert_eq!(state.clipboard.read(), "");

    // A no-op cut chains like any other cut, but it has nothing of its own to
    // add, so the entry stands.
    state.handle_line_cut_tail();
    assert_eq!(state.clipboard.read(), "");
}

#[test]
fn a_chained_cut_reports_that_it_appended() {
    let mut state = state_of("one\ntwo\n");
    state.cursor = at(0, 0);

    state.handle_line_cut_tail();
    assert_eq!(state.message.as_deref(), Some("Cut 3 characters"));

    // The next cut continues the run, so it says it appended.
    state.handle_line_cut_tail();
    assert_eq!(state.message.as_deref(), Some("Appended newline"));

    state.handle_line_cut_tail();
    assert_eq!(state.message.as_deref(), Some("Appended 3 characters"));
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
fn mark_cut_can_be_undone() {
    let mut state = state_of("hello world\n");
    state.cursor = at(0, 6);
    state.handle_mark_set();
    state.cursor = at(0, 11);

    state.handle_mark_cut();
    state.handle_buffer_undo();

    assert_eq!(
        saved_text(&state),
        "hello world\n",
        "the cut region was not restored"
    );
}

#[test]
fn mark_cut_is_one_undo_step_of_its_own() {
    let mut state = state_of("hello world\n");
    state.cursor = at(0, 6);
    state.handle_mark_set();
    state.cursor = at(0, 11);

    state.handle_mark_cut();
    state.finish_editing();
    state.handle_char_insert('!');
    state.finish_editing();

    // The first undo drops the insert; the second, the cut.
    state.handle_buffer_undo();
    assert_eq!(saved_text(&state), "hello \n", "the insert was not undone");

    state.handle_buffer_undo();
    assert_eq!(
        saved_text(&state),
        "hello world\n",
        "the cut was not the step before it"
    );
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
fn cutting_from_the_query_does_not_touch_the_buffers_clipboard() {
    let mut state = state_of("one two\n");
    state.clipboard.write("from the buffer");

    state.handle_search_enter();
    for ch in "query".chars() {
        state.handle_char_insert(ch);
    }
    // The prompt's cursor sits at the end, so move it back to have a tail.
    if let Some(search) = &mut state.search_prompt {
        search.move_cursor_to_start();
    }

    state.handle_search_cut_query();

    assert_eq!(state.search_clipboard.read(), "query");
    assert_eq!(
        state.clipboard.read(),
        "from the buffer",
        "the buffer's clipboard is left alone"
    );
}

#[test]
fn cutting_from_the_query_leaves_the_text_before_the_cursor() {
    let mut state = state_of("one two\n");

    state.handle_search_enter();
    for ch in "query".chars() {
        state.handle_char_insert(ch);
    }
    // The prompt's cursor sits at the end, so move it back into the middle.
    if let Some(search) = &mut state.search_prompt {
        search.move_cursor_to_start();
        search.move_cursor_right();
        search.move_cursor_right();
    }

    state.handle_search_cut_query();

    let search = state.search_prompt.as_ref().expect("the prompt is open");
    assert_eq!(search.query(), "qu", "what was before the cursor stays");
    assert_eq!(state.search_clipboard.read(), "ery");
}

#[test]
fn cutting_an_empty_tail_writes_nothing() {
    let mut state = state_of("one two\n");
    state.search_clipboard.write("an earlier cut");

    state.handle_search_enter();

    state.handle_search_cut_query();

    assert_eq!(
        state.search_clipboard.read(),
        "an earlier cut",
        "an empty query does not wipe the entry"
    );
    assert_eq!(state.message.as_deref(), Some("Nothing to cut"));
}

#[test]
fn the_prompt_pastes_its_own_clipboard_into_the_query() {
    let mut state = state_of("one two\n");
    state.clipboard.write("from the buffer");
    state.search_clipboard.write("two");

    state.handle_search_enter();
    state.handle_clipboard_paste();

    let search = state.search_prompt.as_ref().expect("the prompt is open");
    assert_eq!(
        search.query(),
        "two",
        "the prompt's own contents are what lands"
    );
    assert_eq!(saved_text(&state), "one two\n", "the buffer is untouched");
}

#[test]
fn accepting_a_search_keeps_the_query_for_a_later_prompt_paste() {
    let mut state = state_of("one two one\n");

    state.handle_search_enter();
    for ch in "two".chars() {
        state.handle_char_insert(ch);
    }
    state.handle_search_finish();

    assert_eq!(state.search_clipboard.read(), "two");

    // The entry is for the prompt's own `C-y`: a later search can paste it into
    // its query. The buffer's clipboard is a separate one and stays empty.
    state.handle_search_enter();
    state.handle_clipboard_paste();
    let search = state.search_prompt.as_ref().expect("the prompt is open");
    assert_eq!(search.query(), "two");
    assert_eq!(
        state.clipboard.read(),
        "",
        "the buffer's clipboard never saw the query"
    );
}

#[test]
fn cancelling_a_search_also_keeps_the_query() {
    let mut state = state_of("one two one\n");
    state.cursor = at(0, 4);

    state.handle_search_enter();
    for ch in "two".chars() {
        state.handle_char_insert(ch);
    }
    state.handle_search_cancel();

    assert_eq!(state.search_clipboard.read(), "two");
    assert_eq!(state.cursor, at(0, 4), "the cursor still goes back");
}

#[test]
fn leaving_an_untouched_prompt_keeps_the_previous_query() {
    let mut state = state_of("one two one\n");
    state.search_clipboard.write("an earlier query");

    state.handle_search_enter();
    state.handle_search_cancel();

    assert_eq!(
        state.search_clipboard.read(),
        "an earlier query",
        "typing nothing does not wipe the entry"
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
    // The viewport starts at row 0, which is the row a center of row 2 would
    // use in a 10-row area: the cursor is not yet in the middle, and the cycle
    // moves on from the place it finds rather than centering again.
    let mut state = state_of("one\ntwo\nthree\nfour\nfive\nsix\nseven\n");
    state.cursor = at(6, 0);
    state.handle_view_recenter();

    state.adjust_viewport(area(10, 10));

    assert_eq!(
        state.viewport.row, 1,
        "6 - 10 / 2: row 6 sits in the middle of the ten visible rows"
    );
    assert!(!state.recenter_viewport, "the request is used once");
}

#[test]
fn a_recenter_cycle_visits_center_top_and_bottom() {
    // Each press is consumed by the render that follows it, so the loop lets
    // the adjustment run between presses the way a render would. The place a
    // press lands on is read back from the viewport it left.
    let mut state = state_of(&"a\n".repeat(20));
    state.cursor = at(10, 0);

    let mut rows = Vec::new();
    for _ in 0..4 {
        state.handle_view_recenter();
        state.adjust_viewport(area(5, 10));
        rows.push(state.viewport.row);
    }

    assert_eq!(
        rows,
        vec![8, 10, 6, 8],
        "10 - 5 / 2, then the cursor's own row, then 10 - (5 - 1), and around"
    );
}

#[test]
fn a_top_request_puts_the_cursor_on_the_first_drawn_row() {
    let mut state = state_of(&"a\n".repeat(20));
    state.cursor = at(10, 0);

    state.handle_view_recenter(); // Center, so the cycle is on its first press.
    state.adjust_viewport(area(5, 10));
    state.handle_view_recenter(); // Top.
    state.adjust_viewport(area(5, 10));

    assert_eq!(state.viewport.row, 10, "row 10 is the area's first row");
}

#[test]
fn a_bottom_request_puts_the_cursor_on_the_last_drawn_row() {
    let mut state = state_of(&"a\n".repeat(20));
    state.cursor = at(10, 0);

    for _ in 0..2 {
        state.handle_view_recenter();
        state.adjust_viewport(area(5, 10));
    }
    state.handle_view_recenter(); // Bottom.
    state.adjust_viewport(area(5, 10));

    assert_eq!(
        state.viewport.row, 6,
        "10 - (5 - 1): row 10 is the area's last row"
    );
}

#[test]
fn a_startup_position_leaves_the_cycle_at_its_start() {
    // The startup path asks for a center of its own. The next `C-l` reads the
    // centered viewport and moves on to the top, exactly as a first press does
    // -- so a reader who never pressed `C-l` cannot land in the middle of a
    // cycle they did not start.
    let mut state = state_of(&"a\n".repeat(20));

    state.handle_view_recenter();
    state.adjust_viewport(area(5, 10));
    state.handle_cursor_to_position(10, 0);
    state.adjust_viewport(area(5, 10));
    assert_eq!(state.viewport.row, 8, "the startup path centers");

    state.handle_view_recenter();
    state.adjust_viewport(area(5, 10));

    assert_eq!(state.viewport.row, 10, "the press after it goes to the top");
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
fn the_start_position_centers_the_cursor() {
    let mut state = state_of("one\ntwo\nthree\nfour\nfive\nsix\nseven\n");

    state.handle_cursor_to_position(4, 0);
    state.adjust_viewport(area(4, 40));

    assert_eq!(
        state.viewport.row, 2,
        "4 - 4 / 2, so row 4 sits in the middle of the four visible rows"
    );
}

#[test]
fn the_start_position_centers_the_column_too() {
    let mut state = state_of("0123456789\n");

    state.handle_cursor_to_position(0, 8);
    state.adjust_viewport(area(4, 4));

    assert_eq!(
        state.viewport.col, 6,
        "8 - 4 / 2, so column 8 sits in the middle of the four visible columns"
    );
}

#[test]
fn the_start_position_centering_is_clamped_to_the_buffer_start() {
    let mut state = state_of("one\ntwo\nthree\n");

    // Centering row 0 would ask for row -2, which saturates back to 0.
    state.handle_cursor_to_position(0, 0);
    state.adjust_viewport(area(4, 40));

    assert_eq!(state.viewport, at(0, 0));
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
fn a_cursor_a_screen_and_a_row_out_is_centered() {
    // A 5-row area showing rows 0..5, with the cursor moved straight to row 11
    // -- six rows past the last visible row (4), one more than a whole text
    // area -- so it centers rather than landing against the bottom edge.
    let mut state = state_of(&"a\n".repeat(20));
    state.viewport = at(0, 0);

    state.handle_cursor_to_position(11, 0);
    state.center_viewport = false; // drop the startup center
    state.adjust_viewport(area(5, 10));

    assert_eq!(
        state.viewport.row, 9,
        "11 - 5 / 2: row 11 sits in the middle of the five visible rows"
    );
}

#[test]
fn a_cursor_exactly_a_screen_out_is_not_centered() {
    // Same area, but the cursor is exactly five rows past the last visible row
    // (4), so it touches the threshold without crossing it and keeps the slide.
    let mut state = state_of(&"a\n".repeat(20));
    state.viewport = at(0, 0);

    state.handle_cursor_to_position(10, 0);
    state.center_viewport = false; // drop the startup center
    state.adjust_viewport(area(5, 10));

    assert_eq!(
        state.viewport.row, 6,
        "10 - (5 - 1): the cursor lands on the last visible row"
    );
}

#[test]
fn a_cursor_above_the_viewport_a_screen_out_is_centered() {
    let mut state = state_of(&"a\n".repeat(20));
    state.viewport = at(12, 0);

    // Row 6 is 6 rows above the viewport's row 12, more than the 5-row area.
    state.handle_cursor_to_position(6, 0);
    state.center_viewport = false; // drop the startup center
    state.adjust_viewport(area(5, 10));

    assert_eq!(
        state.viewport.row, 4,
        "6 - 5 / 2: the cursor is centered, not pinned to the top"
    );
}

#[test]
fn an_explicit_recenter_beats_the_far_jump_rule() {
    let mut state = state_of(&"a\n".repeat(20));
    state.viewport = at(12, 0);

    // The request is set and the cursor is a whole screen out, but the explicit
    // `C-l` still wins: it centers by its own rule.
    state.handle_cursor_to_position(0, 0);
    state.adjust_viewport(area(5, 10));

    assert_eq!(state.viewport.row, 0, "row 0 centers as far as it can");
    assert!(!state.recenter_viewport, "the request is used once");
}

#[test]
fn a_far_jump_in_a_zero_height_area_does_not_underflow() {
    let mut state = state_of(&"a\n".repeat(20));
    state.cursor = at(19, 0);

    // An area with no text rows has no threshold to cross, and the arithmetic
    // must stay saturating rather than panicking.
    state.adjust_viewport(area(0, 10));

    assert_eq!(state.viewport.row, 19, "the cursor is at the top");
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

#[test]
fn backspace_at_the_buffer_end_moves_the_cursor_to_the_last_line()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = state_of("abc\nxyz\n");

    state.handle_cursor_buffer_end();
    state.handle_char_delete_backward();

    // The buffer always saves a trailing newline, so backspace there cannot
    // delete one. It leaves the text alone and pulls the cursor back to the
    // end of the last line instead.
    assert_eq!(saved_text(&state), "abc\nxyz\n", "the text is unchanged");
    assert_eq!(
        state.cursor,
        at(1, 3),
        "the cursor is at the last line's end"
    );

    // A second press deletes from that line as usual now that the cursor is
    // on it.
    state.handle_char_delete_backward();
    assert_eq!(
        saved_text(&state),
        "abc\nxy\n",
        "the last character is deleted"
    );
    assert_eq!(state.cursor, at(1, 2), "the cursor follows the deletion");

    Ok(())
}
