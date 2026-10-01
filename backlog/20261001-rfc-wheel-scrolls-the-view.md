# RFC: Let the wheel scroll the view, not the cursor

- Status: implemented

## Summary

A wheel notch today is three `CursorUp`/`CursorDown` steps in a loop. That moves
the cursor and lets `adjust_viewport`'s keep-it-visible rule pull the view along
by the minimum, so the wheel behaves like holding `C-p`/`C-n`: while the cursor
is inside the text area the view does not move at all, and the screen only
slides when the cursor runs off an edge. The reader who just wants to look
further down the file has to drag the cursor to the edge first.

This proposes that the wheel drive the viewport directly instead. The viewport
moves by the notch, and the cursor rides with it, keeping its row on screen --
the cursor stays at the same screen row and the text slides under it. Only at
the buffer's first and last line does the cursor give up that screen row and go
to the edge line, so the view can come to rest with the buffer's first or last
line at the frame's edge rather than stopping a screen short. Once the viewport
is at the edge and the cursor is on the edge line, a further notch is a no-op.

## Motivation

The wheel is the one input that is unambiguously about *looking around*, not
about moving the insertion point. A click sets the cursor where the reader wants
to type; `C-p`/`C-n` walk the cursor a line at a time; the wheel is what a
reader reaches for to see what is above or below without committing to an edit
position. Binding it to cursor motion inverts that: the cursor ends up wherever
the scrolling left it, so a scroll is really a big cursor move, and the view
follows only as a side effect.

The current wiring makes that plain. `handle_scroll` is a loop of cursor steps:

```rust
pub fn handle_scroll(&mut self, rows: isize) {
    if rows < 0 {
        for _ in rows..0 {
            self.handle_cursor_up();
        }
    } else {
        for _ in 0..rows {
            self.handle_cursor_down();
        }
    }
}
```

and the viewport is moved only by [`adjust_viewport`](State::adjust_viewport),
whose ordinary rule is keep-it-visible:

```rust
} else if cursor_pos.row >= self.viewport.row + available_rows {
    self.viewport.row = cursor_pos
        .row
        .saturating_sub(available_rows.saturating_sub(1));
}
```

The result is a scroll that does nothing until the cursor reaches an edge. The
reader scrolls down a few notches with the cursor in the middle of the screen
and sees an unchanged frame, because the view has not yet been asked to move.

There is no established kk behavior to preserve here: the wheel has no meaning
that conflicts with "move the view". This proposal changes only the wheel; the
keyboard moves keep the keep-it-visible scroll they have, because a cursor move
really is about the cursor, and the minimum scroll is what makes holding
`C-n` a slide rather than a re-frame.

## Guide-level explanation

The wheel moves the *view*. The cursor keeps whatever screen row it had, so the
text scrolls under a cursor that appears to stand still:

```text
before (wheel down one notch of three):

   1  alpha            1  alpha
   2  beta             2  beta
   3  gamma   ->       3  gamma
   4  delta ^cursor    4  delta
   5  epsilon          5  epsilon
     ^ viewport       6  zeta
   6  zeta             7  eta
   7  eta              ^ cursor  (still on screen row 4)
     ^ viewport

before (wheel up one notch of three, near the top):

   1  alpha            1  alpha
   2  beta   ->        2  beta
   3  gamma ^cursor    3  gamma  ^cursor
   4  delta            ^ viewport  (clamped at row 0)
     ^ viewport       4  delta
```

Only the buffer's ends break the rule. When the viewport comes to rest at the
first or last line and the cursor cannot keep its screen row without leaving the
buffer, the cursor moves to the edge line instead. This is what lets a reader
scroll all the way to the file's end and see the last line at the bottom of the
frame:

```text
scrolling down to the end (last line is 8):

   5  epsilon            7  eta
   6  zeta      ->       8  theta   <- last line at the frame's last row
   7  eta ^cursor        ^ cursor   (rode down to the edge line)
     ^ viewport
```

Once the viewport is at that edge and the cursor is on the edge line, another
notch in the same direction does nothing, and no message is shown: the cursor
sitting on the first or last line is itself the signal that there is no more to
see.

So a notch always does one of three things, in order:

- **The viewport can still move toward the edge it is heading for.** It moves by
the notch, and the cursor moves with it, keeping its screen row.
- **The viewport has reached the buffer's last line, but the cursor is not on
  that line yet.** The viewport stays at the bottom and the cursor moves toward
  it, giving up its screen row. Only the bottom needs this: screen rows are
  measured from the top of the text area, so a viewport at row `0` already puts
  a visible cursor where its screen row says, with nothing left to give up.
- **Both are at the edge.** Nothing happens.

## Reference-level explanation

### The wheel handler

`handle_scroll` no longer loops over cursor steps. It moves the viewport by the
requested rows and then places the cursor. It needs the text area's size to do
it -- the same argument [`State::adjust_viewport()`] already takes -- because
the viewport is clamped against the drawn height [`State::text_rows()`] leaves,
which is not a buffer-only number:

```rust
pub fn handle_scroll(&mut self, rows: isize, text_area_size: tuinix::Size) {
    let screen_before = self.cursor.row.saturating_sub(self.viewport.row);
    self.scroll_view(rows, text_area_size);
    // ...
}
```

The two phases are one computation. Let `screen_row` be the cursor's row on
screen before the notch, `last_row = self.buffer.rows()` (the row after the last
line, which is what the cursor clamps to), and `last_viewport =
last_row.saturating_sub(drawn_rows - 1)` the furthest the viewport can go with
the text area `drawn_rows` text rows tall.

```text
new viewport = clamp(viewport + rows, 0, last_viewport)
new cursor   = viewport + screen_row                      // phase 1
new cursor   = min(new cursor, last_row)                  // phase 1 clamp
if the viewport clamped at last_viewport, cursor = last_row   // phase 2
```

Phase 1 keeps `cursor - viewport` constant, which is the screen row standing
still. Phase 2 is the bottom-end exception: the viewport stopped at
`last_viewport` while the cursor still had screen rows left under it, so keeping
them would leave the cursor short of the last line and the view stopped a screen
short of the file's end. The cursor gives them up and goes to `last_row`, which
is what lets the last line reach the frame's last row.

Concretely, scrolling down: the viewport moves by `rows`, clamped to
`last_viewport`; the cursor moves by the same `rows`, clamped to `last_row`.
Where the two clamps differ -- the viewport reaching `last_viewport` while the
cursor would still stop short of `last_row` -- phase 2 pulls the cursor down to
the last row.

The top is not the mirror. `screen_row` is measured from the top of the text
area, so when the viewport reaches row `0` the cursor lands on `0 + screen_row`
and is already where phase 1 put it; there is no screen row it cannot keep, and
a cursor that was on the first drawn row simply stays on the first line. The
asymmetry is real, not an oversight: the view's own edge is the file's first
line at the top and the frame's last row at the bottom, and only the bottom edge
can come up short of the last line.

### Keeping the cursor on screen

A viewport moved on its own would be pulled back by `adjust_viewport`'s
keep-it-visible rule on the next render, so the cursor must move with it, as the
current handler's doc comment already says:

> The cursor moves with the viewport because `adjust_viewport()` pulls the
> viewport back to the cursor on the next render, so a viewport moved on its own
> would snap right back.

Because the cursor keeps its screen row, and the row it lands on is inside the
newly shown slice, `adjust_viewport` finds it already visible and leaves the
viewport alone. The two phases above are what makes that true at the ends too:
the cursor is pulled to the edge line precisely so it stays inside the slice the
edge viewport shows.

The height used here is the same [`text_rows()`](State::text_rows) the viewport
is scrolled against everywhere else, so a wheel during an open search prompt
counts the gutter's summary rows out of the height like every other scroll, and
a notch moves the text by three *drawn* rows. The cursor's screen row is
measured over drawn rows, so the summary rows appearing and disappearing as the
hit counts change do not shift where the cursor sits on screen.

### What a notch costs at an end

A notch that changes nothing (both viewport and cursor already at an edge) still
runs the handler, as it does today; the handler simply sets the same values
back. There is no message, and no new state. This matches the rule that a
scroll to a place the view cannot reach is not an error.

### Tests

`tests/state.rs` covers the rule directly: a notch with room to move shifts the
viewport by the notch while the cursor keeps its screen row (down and up); a
notch past the bottom brings the last line to the last drawn row; a notch past
the top keeps the cursor's screen row rather than pulling it to the first line;
a notch with both already at an edge changes nothing. The two older scroll tests
keep their ends (`scrolling_up_stops_at_the_first_line`,
`scrolling_down_stops_at_the_row_after_the_last_line`), now that a notch drives
the viewport.

`tests/e2e_mouse.rs`'s `the_wheel_scrolls_the_view_and_the_cursor_rides_with_it`
drives the real binary: with the cursor on line 2 and the viewport on line 1, one
notch moves the view to line 4 and leaves the cursor on line 5 -- its screen row
unchanged -- and a notch back returns both. The held-cursor case is the part the
old cursor-loop handler could not produce, so the test pins the change rather
than the status quo.

## Alternatives

- **Leave the wheel as cursor motion.** It is the status quo and the smallest
  change, but it makes the wheel something other than a scroll, for the reason
  in the motivation.
- **Move the viewport but do not move the cursor, then let `adjust_viewport`
  pull it back.** The viewport would snap back to the cursor on the next render
  whenever the cursor fell outside the new slice, so the scroll would undo
  itself past a screenful. Moving the cursor with the viewport is what avoids
  that, and is why the current code already does it.
- **Keep the cursor's screen row all the way to the buffer's ends (phase 1
  only).** Simpler, and closer to "the cursor never leaves its screen row", but
  it stops the view a screen short of the file's end: the reader can never bring
  the last line to the frame's edge, because the cursor's fixed screen row
  leaves that many blank rows in the way. The two-phase rule pays a cursor that
  moves to the edge line at the very end in exchange for letting the view reach
  it.

## Open questions

- The exact rounding when the notch would overshoot the edge is settled by the
  arithmetic: the viewport is clamped in one step to `last_viewport` (or `0`),
  so it lands on the edge rather than stepping past and back.
  `tests/state.rs`'s `a_notch_that_reaches_the_end_brings_the_last_line_to_the_bottom`
  and `a_notch_to_the_top_keeps_the_cursors_screen_row` pin both ends.
- Whether `SCROLL_ROWS` (three) is still the right notch, now that a notch moves
  the view rather than nudging the cursor. The value is unchanged by this
  proposal; the decision belongs to a later change that can feel it out.
- A cursor off screen before the notch (which the ordinary rules should not
  produce, but a hand-set viewport can): the rule places it by
  `viewport + screen_row`, and `screen_row` saturates to `0` for a cursor above
  the viewport, so the cursor is placed on the viewport's own row and the next
  `adjust_viewport` corrects anything odd.
