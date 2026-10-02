# Bug: The cursor reaches the row after the last line

- Status: fixed

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

All of the handlers that clamp a cursor row draw the same bound through
`State::row_count()`, which is `TextBuffer::last_row()`. The bug was one wrong
bound copied into `handle_cursor_down`, `handle_cursor_buffer_end`,
`handle_cursor_right`, `handle_cursor_to_screen_position`,
`handle_cursor_to_position`, `handle_buffer_reload`, and `handle_scroll`, and
fixing one would have left the rest.

`handle_char_delete_backward`'s `else if self.cursor.row == self.buffer.rows()`
branch is gone, since the position it handled no longer exists.

The indexing is already right where `rows()` is used as a *count* rather than a
bound -- `State::text_rows` and the `end_row` slice in `src/render.rs` use
`rows()` as an exclusive slice end, which is correct and unchanged.

The automatic recenter in `adjust_viewport` needed one more change: the
centering formula's only floor was the buffer's start, so a jump near the last
line was placed past the buffer's end. Its center is now also floored by the
last page, which is the file-end placement the companion RFC on recentering near
the file end asks for, reached from the bound fixed here.

This is a bug and not an RFC because the code contradicts a documented
invariant in `src/buffer.rs`; the choice is which row the cursor may occupy,
and the buffer's own API already answers it.

## Outcome

Fixed in [#14](https://github.com/sile/kk/pull/14) (merged as `fa47c7b`).

## Outcome

Implemented in PR #14, merged as `fa47c7b`.

Seven cursor handlers clamped against `TextBuffer::rows()`, which is a count
(`last_row + 1`) rather than the last row, so with the cursor at column 0 of the
last line a held `C-n` could land it on `rows()`, a row the buffer does not
have. `TextBuffer::last_row()` now returns `rows().saturating_sub(1)` (an empty
buffer still yields `0`, since it holds one empty line) and `State::row_count()`
exposes it, giving the handlers one boundary to share instead of seven chances
to copy the off-by-one.

`handle_char_delete_backward` lost its special case for the phantom last
position; backspace at the buffer end now joins the last two lines like
anywhere else. `handle_scroll` derives `last_viewport` from `row_count()`.

Unanticipated: the automatic recenter only saturated at the buffer start. A
jump near the end could be placed past `last_viewport`, so the same late-line
jumps kept the cursor off screen even after the handlers were fixed. Clamping
with `centered.min(last_viewport)` gives the tail the mirror of the existing
start behavior.

Coverage: seven tests that pinned the old behavior (cursor down, click below
the buffer, start clamping, end-of-buffer notches, backspace at the end,
reload, and the e2e out-of-range suffix) were updated, plus
`holding_down_stops_on_the_last_line` for the reproduction and
`every_cursor_row_a_handler_can_produce_is_a_real_line`, which drives all seven
paths because a single wrong boundary had been copied around and a per-path
test would not have caught a new copy.

---

## Previous outcome (PR #13, merged as `e52a8b0`)

A wheel notch used to move the cursor by one row and let the viewport follow
only when the cursor walked off the edge, so many notches did nothing and the
view lurched by one row at the end. `handle_scroll` now moves the viewport by
the requested number of rows first and carries the cursor along at the same
screen row, using the same drawn-height clamp as `adjust_viewport`, so a notch
scrolls the summary rows the search prompt puts on screen rather than the raw
buffer rows.

The RFC described the two edges as symmetric, but only the bottom needs a
second phase. A screen row is measured from the top of the text area, so once
the viewport reaches row 0 the cursor already sits at `0 + screen_row` and has
no screen row left to give up; at the bottom the viewport stops at the last
page while the cursor may still have rows below it, so phase two pulls it down
to the last drawn row. The RFC and the rustdoc now record the asymmetry.

Coverage: `tests/state.rs` pins the kept screen row on both edges, the cursor
landing on the last drawn row when the bottom is reached, and the no-ops at
either end. `tests/e2e_mouse.rs` drives the real binary through a notch and
back, which the old implementation could not produce, so the test pins the
change rather than the status quo.

Hits could fall out of the gutter while a summary row was on screen. The totals
were measured over the full text area height while the text was drawn into a
shorter slice -- the one the summary rows leave behind -- so a hit pushed below
the cut belonged to no drawn row and to no total either. The fix measures the
totals over the slice that is actually drawn, and `State::text_rows` now owns
the drawn height, solving the fixed-point between it and the totals; the summary
row count is derived from it instead of being tracked separately.

Coverage: the reported reproduction (`tests/render.rs`, a hit on row 4 counted
in the bottom total) plus a `noprop` property test asserting that drawn rows,
the top total, and the bottom total always add up to the whole hit count across
random buffers, viewports, and area heights. Reverting the fix makes the new
tests fail with the totals off by the displaced hits.

`C-l` is now bound in the search mode, so a hit can be repositioned without
leaving the prompt. The key runs the same cycle as in the edit mode (see the
sibling note on cycling through positions), which is only possible because the
next place is derived from the viewport rather than remembered per mode: a press
in either mode continues through the same three places with nothing to carry
across. The `C-l recenter` row sits right after `C-s next` in the search legend,
and the box width is unchanged.

The place is chosen inside the fixed-point loop that settles the summary rows,
rather than early-returning, because the gutter's summary rows change the text
area's height. This turned up a real one-row bug in search mode: the third press
could name "bottom" and leave the hit one row past the last drawn row. A
regression test covers it.

Unanticipated: the search legend row did not widen the box, and existing search
steps were already unaffected -- they leave any recenter request untouched, so
they still keep the hit minimally visible rather than jumping to the center.

Coverage: `tests/search.rs` presses `C-l` three times with the summary rows in
play and checks the hit reaches the last drawn row; `tests/binding.rs` covers
the new binding and `tests/legend.rs` the legend row.

The scope is unchanged from what is described above.
