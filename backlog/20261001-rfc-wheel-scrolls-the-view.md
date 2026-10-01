# RFC: Let the wheel scroll the view, not the cursor

- Status: open

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
- **The viewport has reached the buffer's first or last line, but the cursor is
  not on that line yet.** The viewport stays at the edge and the cursor moves
toward it, giving up its screen row.
- **Both are at the edge.** Nothing happens.

## Reference-level explanation

### The wheel handler

`handle_scroll` no longer loops over cursor steps. It moves the viewport by the
requested rows and then places the cursor:

```rust
pub fn handle_scroll(&mut self, rows: isize) {
    self.scroll_view(rows);
}
```

The two phases are one computation. Let `screen_row` be the cursor's row on
screen before the notch, `last_row = self.buffer.rows()` (the index of the last
line), and `last_viewport = last_row.saturating_sub(text_height - 1)` the
furthest the viewport can go with the text area `text_height` tall.

```text
new viewport = clamp(viewport + rows, 0, last_viewport)
new cursor   = clamp(viewport + screen_row, 0, last_row)   // phase 1
if the viewport hit an end and the cursor did not reach it, pull the cursor in
                                                          // phase 2
```

Phase 1 keeps `cursor - viewport` constant, which is the screen row standing
still. Phase 2 is the end exception: when the viewport clamped at `0` while the
cursor still has rows to give up, `cursor = viewport + screen_row` may still be
above `0`, so the cursor is clamped to the edge too -- but only after the
viewport has stopped, which is what produces the ride-to-the-edge rather than a
stall a screen short.

Concretely, scrolling down: the viewport moves by `rows`, clamped to
`last_viewport`; the cursor moves by the same `rows` unless that would take it
past `last_row`, in which case it stops at `last_row`. The difference between
the two clamps is exactly the second phase. Scrolling up is the mirror, with
`0` for both clamps.

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

- The exact rounding when the notch would overshoot the edge: clamp the move to
  land exactly on the edge rather than stepping past and back. The handler
  computes the clamp in one step, so this falls out of the arithmetic, but it
  should be pinned by a test.
- Whether `SCROLL_ROWS` (three) is still the right notch, now that a notch moves
  the view rather than nudging the cursor. The value is unchanged by this
  proposal.
- A cursor off screen before the notch (which the ordinary rules should not
  produce, but a hand-set viewport can): the rule above simply places it by
  `viewport + screen_row` clamped to the buffer, and the next `adjust_viewport`
  corrects anything odd. Worth a note in the fix rather than a design here.
