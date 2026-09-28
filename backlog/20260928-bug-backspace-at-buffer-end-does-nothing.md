# Bug: Backspace at the buffer end does nothing

- Status: open

## Summary

With the cursor at the buffer end, backspace deletes nothing. The buffer end
is a real cursor position — `C-x C-e` (and scrolling to the bottom) put the
cursor on the row after the last line, at column 0 — but
`TextBuffer::delete_char_before` finds nothing to delete there, so the trailing
newline is not removed and the last line is not joined onto the previous one.

## Reproduction

Open a buffer whose last line is non-empty (any file not ending in a blank
line), move to the buffer end, and press backspace.

```text
file: "abc\nxyz\n"
key:  C-x C-e        (Action::CursorBufferEnd)
key:  <BACKSPACE>    (Action::CharDeleteBackward)
observed: nothing changes; the buffer is still "abc\nxyz\n"
expected: the buffer becomes "abc\nxyz"
```

The same call can be made directly:

```rust
let mut state = kk::State::new(kk::TextBuffer::new("abc\nxyz\n"));
state.handle_cursor_buffer_end();
state.handle_char_delete_backward();
assert_eq!(state.buffer.to_text(), "abc\nxyz"); // fails: still "abc\nxyz\n"
```

## Observed behavior

`State::handle_cursor_buffer_end` in `src/state.rs` sets the cursor to
`(row: self.buffer.rows(), col: 0)`:

```rust
pub fn handle_cursor_buffer_end(&mut self) {
    self.cursor.row = self.buffer.rows();
    self.cursor.col = 0;
    self.finish_editing();
}
```

`rows()` is the number of lines, so `cursor.row == rows()` is one past the last
line — a row that does not exist. `handle_char_delete_backward` then calls
`TextBuffer::delete_char_before(self.cursor)`, which handles the
column-0 case by joining the line at `pos.row` onto the line above it:

```rust
} else if pos.row > 0 {
    let current_line = self.text.get(pos.row).cloned();
    if let Some(current_line) = current_line {
        ...
```

For the buffer end, `self.text.get(pos.row)` is out of range and returns
`None`, so the `if let` body never runs and `delete_char_before` returns
`None`. The cursor is left where it was and nothing is deleted.

The position is not degenerate or unreachable: `C-x C-e` places the cursor
there directly, and `scrolling_down_stops_at_the_row_after_the_last_line` in
`tests/state.rs` documents the same row as a valid cursor position reached by
scrolling. The invariant that is violated is the one `delete_char_before`
documents: "At the start of a line, this line is joined onto the previous
one" — the buffer end is the start of the (virtual) line after the last one,
and nothing is joined.

## Expected behavior

Backspace at the buffer end should delete the trailing newline, joining the
last line onto the previous one, exactly as backspace at the start of any other
line does. `delete_char_before` documents that a column-0 position joins the
line at `pos.row` onto the line above; the buffer end should behave the same
way with the last line as the line above.

## Impact

A correctness problem reachable from the public API: a common editing action
silently does nothing at a position a user reaches with a documented binding.
The user sees backspace ignored and has no way to delete the trailing newline
from that position. No resource effect.

## Notes

The root cause is in `TextBuffer::delete_char_before`, not in the handler: the
handler places the cursor at a valid position (`rows()`, `0`), and the buffer
method fails to treat `pos.row == self.text.len()` as "one past the last line".
A fix should make `delete_char_before` handle `pos.col == 0` with
`pos.row == self.text.len()` as a join of the last line onto the line above,
without widening the row range the method accepts for other edits. The failing
behavior is covered by
`backspace_at_the_buffer_end_joins_the_last_line_onto_the_previous_one` in
`tests/state.rs`.
