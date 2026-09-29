# Bug: The search cursor is invisible at the end of a line

- Status: open

## Summary

While a search prompt is open, the buffer cursor is drawn in the text area as
reversed video so the reader can see where a hit step has put it. The
reverse-video cursor is painted on the character under the cursor, so when the
cursor sits at the end of a line -- one column past the last character, where
there is no character to paint -- nothing is reversed and the cursor vanishes.
The reader sees no cursor in the text area until they step to a hit that is not
at the end of its line.

## Reproduction

Open a buffer and put the cursor at the end of a line, then open a search
prompt. The mismatch is between the cursor position and the characters the
renderer iterates over.

```text
file: "one\n"
key:  <END>          (cursor to column 3, the end of "one")
key:  C-s             (Action::SearchStart, the prompt opens)
observed: the text area paints "one" plainly; no reversed cell marks the cursor
          at column 3
expected: column 3 is painted reversed, a dummy cursor one cell wide, so the
          cursor is visible at the end of the line as it is mid-line
```

The same state can be built directly:

```rust
let mut state = kk::State::new(kk::TextBuffer::new("one\n"));
state.cursor = kk::TextPosition { row: 0, col: 3 };
state.search_prompt = Some(kk::SearchPrompt::new());
let mut frame = tuinix::Frame::new(tuinix::Size { rows: 1, cols: 5 });
kk::render_text_area(&state, &mut frame);
// The cell at (0, 3) keeps Style::RESET: nothing was painted there.
```

## Observed behavior

`render_line` in `src/render.rs` loops over the line's characters and, for each
one, reverses the cell when it is the cursor:

```rust
for (current_col, ch) in line.char_cols() {
    if current_col >= start_col {
        ...
        let is_cursor = state.search_prompt.is_some() && pos == state.cursor;
        ...
    }
}
```

The loop body only runs for columns that hold a character. The end-of-line
position (`col == line width`) holds none, so `pos == state.cursor` is never
true there and no cell is reversed. The cursor is a real position -- `End`,
`C-e`, `C-f` past the last character, and a search hit or a click at the line's
end all reach it -- so the renderer's contract that a mark and the search
cursor are "both reversed, so the cursor stands out as plainly as a mark"
(`render_text_area`'s rustdoc) is violated for exactly the columns where the
cursor has nothing to sit on.

## Expected behavior

While a search prompt is open, the buffer cursor should be painted reversed
wherever it sits, including one cell past the end of a line. A cursor at the
end of a line should show as a single reversed blank cell at that column, the
same way an editor shows a block cursor past the last character. The mark
handling has the same shape and is already correct: a marked range extends to
`usize::MAX` for its end column, so a mark that runs to the end of a line
reverses the cells up to it regardless of where the characters stop.

## Impact

An ergonomics problem, not a correctness one: no edit is lost and no documented
invariant of the buffer is broken, but the reader cannot see where a search step
or a click has put the cursor whenever that position is the end of a line. It is
reproducible from the public renderer with the state above. No resource effect.

## Notes

The fix belongs in `render_line`: after the character loop, when a search
prompt is open and `state.cursor` is on this row at a column at or past
`start_col` and past the last character, paint a reversed space at that column.
The same gap exists for a wide character cut by the viewport, but that is a
different question and should not be folded in. The e2e
`the_legend_hides_for_the_buffer_cursor_while_the_prompt_is_open` and the unit
`a_search_reverses_the_character_under_the_cursor` both cover the mid-line case
and must keep passing; a new case for the end-of-line column is what is missing.
