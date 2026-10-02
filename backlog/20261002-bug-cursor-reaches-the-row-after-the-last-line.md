# Bug: The cursor reaches the row after the last line

- Status: open

## Summary

In a buffer of ten lines the cursor can be moved onto an eleventh row that does
not exist. `TextBuffer::rows()` is documented as "the number of lines", and the
last line is therefore index `rows() - 1`, but the cursor handlers clamp the
cursor to `rows()` instead and treat that row as a real position. The row is
then rendered as a virtual blank line below the text, and every handler that
measures a position against `rows()` agrees on it, so the cursor sticks there.

## Reproduction

Open a file with ten lines and hold the down arrow to the bottom, or click the
first row of blank space below the text.

```text
file:  "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"
key:   <DOWN> x 10    (Action::CursorDown)
observed: the cursor rests on row 10, an eleventh row drawn below "10"
expected: the cursor stops on row 9, the last line "10"
```

The same row is reachable from the public API:

```rust
let mut state = kk::State::new(kk::TextBuffer::new("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"));
assert_eq!(state.buffer.rows(), 10);
for _ in 0..10 {
    state.handle_cursor_down();
}
assert_eq!(state.cursor.row, 9); // fails: cursor.row is 10
```

## Observed behavior

`State::handle_cursor_down` in `src/state.rs` clamps to `rows()`:

```rust
pub fn handle_cursor_down(&mut self) {
    self.cursor.row = self.cursor.row.saturating_add(1).min(self.buffer.rows());
    self.finish_editing();
}
```

`rows()` is the number of lines, so `rows()` is one past the last line, and the
cursor comes to rest there. The same one-past-the-end row is written in every
other handler that takes a cursor row from outside:

- `handle_cursor_buffer_end` sets `self.cursor.row = self.buffer.rows()`.
- `handle_cursor_right` moves to the next line while `self.cursor.row < self.buffer.rows()`, so pressing right at the end of the last line enters the row.
- `handle_cursor_to_screen_position` clamps a click to `self.buffer.rows()`, so a click on the blank space under the text lands on it.
- `handle_cursor_to_position` and `handle_buffer_reload` clamp to `self.buffer.rows()` too.

`handle_scroll` names the same row `last_row` and lets the cursor settle on it
when the viewport reaches the bottom. The row is not degenerate as far as the
rest of the buffer is concerned: `cols(rows())` returns `0` and
`adjust_to_char_boundary` leaves it at column `0`, exactly like an empty line,
so nothing rejects it.

What the behavior breaks is the meaning of `rows()` itself. The rustdoc on
`TextBuffer::rows()` says it returns "the number of lines", and
`TextBuffer::line(row)` returns `None` for `row == rows()` ("or `None` if there
is no such line"), so `rows()` is a row that both accessors say does not exist.
The cursor's row is documented as a line index, and every such index is below
`rows()`; a cursor at `rows()` is outside the range the buffer's own API defines
for it.

## Expected behavior

The cursor should stop at the last real line, index `rows() - 1`, which is what
`TextBuffer::line()` and `TextBuffer::cols()` accept and what the rest of the
buffer documents as a valid row. `rows()` is "the number of lines", not a line
that can be addressed, so no cursor handler should place the cursor there:

```rust
let mut state = kk::State::new(kk::TextBuffer::new("1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n"));
for _ in 0..10 {
    state.handle_cursor_down();
}
assert_eq!(state.cursor.row, state.buffer.rows() - 1);
```

A buffer that always saves a trailing newline (see `TextBuffer::to_text`) has
its newline *after* the last line, not on a line of its own; the row after the
last line is not a place a cursor can stand.

## Impact

A correctness problem reproducible from the public API: a cursor position that
no line backs is reachable by an ordinary keypress and by a click, and the
editing handlers have to special-case it to avoid misbehaving. The class of
special cases it creates is what `backspace-at-buffer-end-does-nothing` had to
work around: `handle_char_delete_backward` carries a branch for
`cursor.row == self.buffer.rows()` that exists only because the row is
reachable. Fixing the row bound removes the reason for that branch. No resource
effect.

## Notes

All of the handlers that clamp a cursor row should draw the same bound
(`rows() - 1`, or `rows().saturating_sub(1)` so an empty buffer behaves), not
just `handle_cursor_down`: the bug is one wrong bound copied into
`handle_cursor_down`, `handle_cursor_buffer_end`, `handle_cursor_right`,
`handle_cursor_to_screen_position`, `handle_cursor_to_position`,
`handle_buffer_reload`, and `handle_scroll`, and fixing one leaves the rest.
`handle_char_delete_backward`'s `else if self.cursor.row == self.buffer.rows()`
branch should then be dropped, since the position it handles no longer exists.

The indexing is already right where `rows()` is used as a *count* rather than a
bound -- `State::text_rows` and the `end_row` slice in `src/render.rs` use
`rows()` as an exclusive slice end, which is correct and should not change.

This is a bug and not an RFC because the code contradicts a documented
invariant in `src/buffer.rs`; the choice is which row the cursor may occupy,
and the buffer's own API already answers it.
