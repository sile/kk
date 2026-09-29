# RFC: Recenter when the cursor jumps far off screen

- Status: draft

## Summary

When the cursor leaves the text area, `adjust_viewport` scrolls by the minimum
needed to bring it back -- one row if it moved one row out, fifty if it jumped
fifty. That is right for a cursor that walked off the edge: the reader is
following a move they just made, and the least disruption is to slide the
screen by the same amount. It is wrong for a cursor that jumped somewhere far
away: the cursor lands one row inside the edge, with almost none of its
surroundings visible, and the reader has to scroll by hand to see where they
are.

This proposal keeps the minimum scroll for small excursions and recenters
instead once the cursor is far enough off screen. The threshold is a single
number compared against how far the cursor is outside the viewport, so it
applies to every cursor move at once -- `C-p`/`C-n`, `C-b`/`C-f` word and
buffer moves, the search steps, and clicks -- rather than adding a rule per
caller.

## Motivation

`adjust_viewport` has one rule for a cursor outside the viewport: put it
against the nearest edge.

```rust
// src/state.rs, adjust_viewport
if cursor_pos.row < self.viewport.row {
    self.viewport.row = cursor_pos.row;
} else if cursor_pos.row >= self.viewport.row + available_rows {
    self.viewport.row = cursor_pos
        .row
        .saturating_sub(available_rows.saturating_sub(1));
}
```

The rule is the same whether the cursor moved one row or a thousand, and the
result is the same shape either way: the cursor ends up flush against an edge.

For a small move that is what the reader wants. Holding `C-n` to walk down a
screen should slide the text up one row at a time, not re-frame the window on
every row. The same holds for a search step to a hit just below the last
visible row: the reader is walking through hits, and a screen that jumps around
them makes the walk hard to follow. The recent change to the search steps took
the minimum scroll side of this for granted, for exactly that reason.

The same rule misfires when the cursor *jumps*. `C-x C-e` (buffer end) can move
the cursor thousands of rows at once. So can `C-v`-style paging, a search step
to a hit far down the file, and a click far from the cursor. In each case the
cursor lands on the last visible row (or the first), its surrounding lines are
just past the edge, and the reader's next action is almost always to scroll
toward what they jumped to. Centering at the moment of the jump would have put
the destination in the middle of the screen, which is the most useful place for
a position the reader did not already have in view.

So the two moves want opposite things, and the amount of scroll they ask for is
what tells them apart:

- a cursor a row or two outside the viewport is *arriving* from inside it; the
  reader has the context on screen and a small slide preserves it;
- a cursor far outside it was *not* on screen at all; the reader has no context
  for it, and pinning it to an edge shows the least of it.

The rule below draws the line at a single threshold, so both cases keep coming
from the one place that already decides scrolling, and no caller needs to say
which kind of move it is.

## Guide-level explanation

A cursor that leaves the text area brings the screen with it, by one of two
amounts:

- **Near the edge.** It scrolls just far enough to show the cursor, as it does
today. The cursor ends up against the edge it came in through.
- **Far from the edge.** It scrolls to put the cursor in the middle of the text
  area instead.

"Near" is measured in how far the cursor is *outside* the viewport, not in how
far it moved: a cursor one row below the last visible row is one row out and
takes the small slide, however it got there; a cursor a screen-height below is
far out and centers. Walking off the edge with `C-n` therefore keeps the
one-row slide, and so does a search step to a hit just past the last row.

A jump that lands more than half a text area away from the viewport centers:

```text
before:                              after C-x C-e (jump far down):

   1  alpha                            90  ...
   2  beta                             91  eta    <- centered
   3  gamma          ->                92  theta
   4  delta                            93  iota
     ^ viewport                          ^ cursor
     ^ cursor

before:                              after C-n (walk one row out):

   1  alpha                            2  beta
   2  beta           ->               3  gamma
   3  gamma                           4  delta
   4  delta                            5  epsilon   <- one-row slide
     ^ viewport, cursor on 4              ^ cursor on 5
```

The screen does not move at all when the cursor is already visible, as today.
Explicit recenter requests (`C-l`, the startup position) still center
unconditionally and are not affected by the threshold: those say "put my cursor
in the middle" as a command, and this proposal is only about what the automatic
rule does when no one asked.

## Reference-level explanation

### Where the threshold lives

The rule is one more branch inside `adjust_viewport`, before the existing
keep-it-visible code and after the unconditional recenter request. The viewport
and the text area size are both already there, so nothing has to be threaded in
from a caller:

```rust
// after the `if self.recenter_viewport` block
let too_far = ROW_THRESHOLD; // see below

if cursor_pos.row < self.viewport.row {
    let rows_out = self.viewport.row - cursor_pos.row;
    if rows_out > too_far {
        self.viewport.row = cursor_pos.row.saturating_sub(available_rows / 2);
    } else {
        self.viewport.row = cursor_pos.row;
    }
} else if cursor_pos.row >= self.viewport.row + available_rows {
    let rows_out = cursor_pos.row - (self.viewport.row + available_rows).saturating_sub(1);
    if rows_out > too_far {
        self.viewport.row = cursor_pos.row.saturating_sub(available_rows.saturating_sub(available_rows / 2));
    } else {
        self.viewport.row = cursor_pos
            .row
            .saturating_sub(available_rows.saturating_sub(1));
    }
}
```

Centering is the same arithmetic the recenter branch already uses
(`cursor_pos.row.saturating_sub(available_rows / 2)`), recomputed against the
*new* cursor rather than the old one, so the plain-recenter and
threshold-recenter paths land on the same viewport when they fire.

### The threshold value

The threshold is a property of the text area, not a fixed row count, so it
keeps its meaning as the terminal resizes. Two candidates:

- **Half a text area.** Centering then replaces a scroll of more than half a
  screen. A cursor just past the middle of the screen is `avail/2` out and still
  near, because centering it would move the text about as far as pinning it
  would; only past that does centering clearly win.
- **One whole text area.** The cursor centers only when it was not on screen at
  all *and* is roughly a screen away, which matches "the reader had no context
  for it". Nearer than that is treated as a walk.

This RFC proposes **half a text area**: it is the smallest threshold that still
captures the jump-far case, and it is the point where centering stops being the
larger move. Whichever value is picked, it should be a named constant beside the
rule, not a literal at the comparison.

### Horizontal

Only rows are proposed here. Columns already have the same minimal-scroll rule,
and a long line can put the cursor many columns out, but horizontal centering
has its own history -- it hides the start of the line even for a small move, so
recentering horizontally is a separate question. Leaving columns at the minimum
scroll keeps this change to one axis and one branch, and the same threshold can
apply to columns later without a new mechanism.

### Interaction with explicit recenter

The `recenter_viewport` branch runs first and returns, so an explicit request
still centers no matter how close the cursor is. The threshold only decides the
automatic case, which is what the readers of `handle_search_next_hit` and the
plain moves already expect.

The mouse handlers call `adjust_viewport` through the same path, so a click far
from the cursor centers under this rule without any mouse-specific code.

### Tests

The existing minimal-scroll tests assert the behavior this changes for a *far*
cursor, so those cases move to asserting centering, and a new pair is needed:

- a cursor one row outside the viewport still scrolls by one row (the walk
  case must not regress);
- a cursor more than half a screen outside scrolls so it is centered;
- the boundary: exactly `avail/2` rows out stays minimal, `avail/2 + 1`
  centers;
- an explicit `C-l` still centers even when the cursor is one row out
  (the threshold branch must not run).

## Drawbacks

- **A second scrolling mode.** Today "the cursor is visible and the viewport
  follows" is one rule with one exception (the explicit request). This adds a
  second automatic mode whose boundary is a number the reader has to feel out.
  A cursor 5 rows out does something different from a cursor 6 rows out, and
  neither is obvious from the key that moved it.
- **The threshold may be the wrong shape for some moves.** Paging commands are
  a jump *by intent*: `C-v` moves the cursor about a screen, so it lands near
  or past the threshold depending on the text area, and the same key can center
  on one terminal size and slide on another.
- **Centering on a jump can still surprise.** A reader who jumps with `C-x C-e`
  to read the end of a file may want the cursor at the edge (so the text above
  is visible) rather than centered. The rule cannot tell "read from here" apart
  from "go look at here".
- **More to test.** The viewport rule is touched by nearly every handler's
  tests; a threshold adds boundary cases to that surface.

## Rationale and alternatives

- **Do nothing.** The minimal scroll is simple and predictable, and a far jump
  is rare. Rejected: the far jump is exactly when the reader most needs context,
  and "the cursor is against an edge" is the least informative place to show a
  destination.

- **Recenter on every off-screen move, with no threshold.** This is the
  sibling `recenter-cycles-through-positions` family's "always center" pole. It
  is simpler (one rule for all off-screen cursors) but it makes the very walk
  the search-step change was meant to protect jump on every row. Rejected for
  that reason.

- **Per-caller choice** (the search steps recenter, plain moves do not, paging
  does). This is what kk has today in spirit, since every caller writes the
  flag. Rejected because it multiplies rules per command and gives no answer for
  a new command: the reader has no model to predict from.

- **A fixed row count instead of a fraction of the screen.** Easier to state
  ("more than 5 rows out centers") but it means different things on a 10-row and
  a 60-row terminal, and a big terminal would center for moves that are a small
  fraction of it. Rejected in favor of a fraction.

- **"Always keep the destination visible, and let the reader recenter by
  hand."** Closest to the current behavior, and the explicit `C-l` already
exists for the rest. Rejected: it puts the burden on the reader at the moment
  they are least oriented.

## Unresolved questions

- The threshold itself: half a text area (proposed here), a whole one, or
  something else. This must be settled before implementation, and it is the one
  number the proposal cannot justify from first principles -- the trade is
  between how much disruption centering is worth and how big a "small" move is.
- Whether the threshold applies to the *distance the cursor moved* or the
  *distance it is outside the viewport*. This RFC uses the latter ("how far out
  is it"), which is what `adjust_viewport` can see and what keeps a walk from
  crossing the threshold one row at a time. Using the move distance would need
  every handler to pass it in (or keep a last-cursor field) and would center a
  fast walk that happened to skip rows. Worth confirming no case needs the move
  distance instead.
- Whether columns should use the same threshold (see the Horizontal section)
  or stay minimal for now.
- Whether paging-style commands (`C-v`, `M-v`) should be able to opt out and
  always center, since their whole purpose is a jump even when the arithmetic
  does not cross the threshold. That would mean an intent flag, which this RFC
  avoids everywhere else.

## Future possibilities

The threshold and a future `Recenter` enum
(`20260928-rfc-recenter-cycles-through-positions.md`) both describe where a
cursor should end up, so they may eventually share a home: the enum names the
place, the threshold decides when to pick `Center` automatically. Keeping this
RFC to the automatic rule means that merge, if it happens, is additive.

A later `scroll-margin` (keep a few rows of context past the cursor and center
before the cursor reaches the edge) is the natural generalization: it is this
proposal's small-move side applied all the time, and it would also stop a walk
from pinning the cursor to the edge. It is a bigger change to the same rule and
is not proposed here.
