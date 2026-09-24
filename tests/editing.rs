//! Properties and examples of text edits and character boundaries.

use std::cell::Cell;

/// A buffer whose whole text is one line of `cols` display cells, filled from
/// a two-character alphabet so that wide characters appear.
///
/// An empty line has no row at all, so the sampled length starts at 1.
fn sample_line(ctx: &mut noprop::TestCaseContext) -> String {
    let len = noprop::sample_usize_in(ctx, 1..=8);
    let mut text = String::new();
    for _ in 0..len {
        text.push(noprop::sample_choice(ctx, &['a', '\u{3042}']));
    }
    text
}

#[test]
fn a_column_inside_a_wide_character_snaps_to_its_edges() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);
    let wide = Cell::new(0usize);
    let inside = Cell::new(0usize);

    runner.run(256, |ctx| {
        let text = sample_line(ctx);
        let buffer = kk::TextBuffer::from_text(&text);
        let cols = buffer.cols(0);
        let col = noprop::sample_usize_in(ctx, 0..=cols + 2);
        let pos = kk::TextPosition { row: 0, col };

        let floor = buffer.adjust_to_char_boundary(pos, true);
        let ceil = buffer.adjust_to_char_boundary(pos, false);

        assert_eq!(floor.row, 0);
        assert_eq!(ceil.row, 0);
        // Snapping never crosses the input column: flooring lands at or before
        // it, ceiling at or after it. Past the last character start there is
        // no boundary to snap to in either direction, so both clamp to the
        // line's width instead.
        let past_end = col > cols;
        assert!(
            floor.col <= col,
            "floor of {col} landed at {} for {text:?}",
            floor.col
        );
        assert!(
            floor.col <= cols,
            "floor of {col} passed {cols} for {text:?}"
        );
        if past_end {
            assert_eq!(ceil.col, cols, "ceil of {col} for {text:?}");
        } else {
            assert!(
                ceil.col >= col,
                "ceil of {col} landed at {} for {text:?}",
                ceil.col
            );
            assert!(ceil.col <= cols, "ceil of {col} passed {cols} for {text:?}");
        }

        // A column that sits strictly inside a wide character is the one case
        // where the two directions differ: the character starts two columns
        // below, so flooring snaps back to that start and ceiling snaps past
        // the character's end. A column that already is a boundary snaps to
        // itself both ways, which is why "inside" is tested, not "is wide".
        let inside_wide = col % 2 == 1
            && col < cols
            && buffer.text[0].char_at_col(col).is_none()
            && buffer.text[0].char_at_col(col - 1) == Some('\u{3042}');
        if inside_wide {
            inside.set(inside.get() + 1);
            assert_eq!(floor.col, col - 1, "floor of column {col} for {text:?}");
            assert_eq!(
                ceil.col,
                col + 1,
                "ceil of the wide char's trailing cell {col} for {text:?}"
            );
        } else if buffer.text[0].char_at_col(col).is_some() || col == cols {
            // A real boundary is a fixed point of both directions.
            assert_eq!(floor.col, col, "floor of the boundary {col} for {text:?}");
            assert_eq!(ceil.col, col, "ceil of the boundary {col} for {text:?}");
        }
        if text.contains('\u{3042}') {
            wide.set(wide.get() + 1);
        }
        Ok(())
    })?;

    assert!(
        wide.get() > 0,
        "no case contained a wide character\n{runner}"
    );
    assert!(inside.get() > 0, "no case landed inside one\n{runner}");
    Ok(())
}

#[test]
fn inserting_then_deleting_before_restores_the_text() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let text = sample_line(ctx);
        let inserted = noprop::sample_choice(ctx, &['x', 'z', '\u{3042}']);
        let mut buffer = kk::TextBuffer::from_text(&text);
        let char_index = noprop::sample_usize_in(ctx, 0..=buffer.text[0].0.len());
        let col = buffer.col_at_char_index(0, char_index).expect("row 0");
        let at = kk::TextPosition { row: 0, col };

        let after_insert = buffer.insert_char_at(at, inserted);
        assert_eq!(
            after_insert.col,
            col + kk::char_cols(inserted),
            "cursor after inserting {inserted:?} into {text:?}"
        );

        let restored = buffer.delete_char_before(after_insert);

        assert_eq!(restored, Some(at), "cursor after deleting {inserted:?}");
        assert_eq!(
            buffer.to_text(),
            format!("{text}\n"),
            "text after the round trip"
        );
        assert!(buffer.dirty, "an edit marks the buffer dirty");
        Ok(())
    })?;

    Ok(())
}

#[test]
fn char_index_at_col_inverts_col_at_char_index() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let text = sample_line(ctx);
        let buffer = kk::TextBuffer::from_text(&text);
        let line = &buffer.text[0];

        for index in 0..=line.0.len() {
            let col = buffer.col_at_char_index(0, index).expect("row 0");
            assert_eq!(
                buffer.char_index_at_col(0, col),
                Some(index),
                "column {col} of {text:?}"
            );
        }
        Ok(())
    })?;

    Ok(())
}

#[test]
fn a_wide_character_occupies_two_columns() {
    let mut buffer = kk::TextBuffer::from_text("\u{3042}x\n");

    assert_eq!(buffer.cols(0), 3, "one wide char and one narrow char");
    assert_eq!(buffer.char_index_at_col(0, 2), Some(1));
    assert_eq!(buffer.col_at_char_index(0, 1), Some(2));
    assert_eq!(
        buffer.text[0].char_at_col(1),
        None,
        "column 1 is the wide char's trailing cell, not a character start"
    );

    // Inserting at column 1 goes in *after* the wide character. Column 1 is
    // the wide character's trailing cell, and the insertion point is found by
    // accumulating cell widths, so a column that is *inside* a character falls
    // on the far side of it. Column 2, the character's own start, goes before.
    let inserted = buffer.insert_char_at(kk::TextPosition { row: 0, col: 1 }, 'a');
    assert_eq!(buffer.to_text(), "\u{3042}ax\n");
    // The cursor is placed by adding the inserted width to the *given* column,
    // not to the column the character actually went in at (2), so it comes
    // back one cell short of the text's width.
    assert_eq!(inserted.col, 2);
    assert_eq!(buffer.cols(0), 4);

    // The same insertion at the wide character's own start column goes before
    // it, which shows the two columns 1 and 2 disagree about the split.
    let mut at_start = kk::TextBuffer::from_text("\u{3042}x\n");
    at_start.insert_char_at(kk::TextPosition { row: 0, col: 2 }, 'a');
    assert_eq!(
        at_start.to_text(),
        "\u{3042}ax\n",
        "column 2 is past the wide char"
    );
    let mut at_zero = kk::TextBuffer::from_text("\u{3042}x\n");
    at_zero.insert_char_at(kk::TextPosition { row: 0, col: 0 }, 'a');
    assert_eq!(
        at_zero.to_text(),
        "a\u{3042}x\n",
        "column 0 is the wide char's start"
    );

    // Backspacing from column 2 snaps back to the wide character's start and
    // deletes it.
    let before = buffer.delete_char_before(kk::TextPosition { row: 0, col: 2 });
    assert_eq!(before, Some(kk::TextPosition { row: 0, col: 0 }));
    assert_eq!(
        buffer.to_text(),
        "ax\n",
        "backspacing at column 2 cuts the wide char, not the one after it"
    );
}

#[test]
fn insert_newline_at_splits_the_line() {
    let mut buffer = kk::TextBuffer::from_text("abcd\n");

    let at = buffer.insert_newline_at(kk::TextPosition { row: 0, col: 2 });

    assert_eq!(at, kk::TextPosition { row: 1, col: 0 });
    assert_eq!(buffer.rows(), 2);
    assert_eq!(buffer.to_text(), "ab\ncd\n");
}

#[test]
fn insert_char_past_the_end_pads_with_empty_lines() {
    let mut buffer = kk::TextBuffer::from_text("a\n");

    let at = buffer.insert_char_at(kk::TextPosition { row: 3, col: 0 }, 'b');

    assert_eq!(at, kk::TextPosition { row: 3, col: 1 });
    assert_eq!(buffer.rows(), 4);
    assert_eq!(buffer.to_text(), "a\n\n\n b\n".replace(' ', ""));
}

#[test]
fn deleting_at_the_end_of_a_line_joins_the_next_one() {
    let mut buffer = kk::TextBuffer::from_text("ab\ncd\n");

    let deleted = buffer.delete_char_at(kk::TextPosition { row: 0, col: 2 });

    assert!(deleted);
    assert_eq!(buffer.rows(), 1);
    assert_eq!(buffer.to_text(), "abcd\n");
}
