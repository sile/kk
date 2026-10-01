# RFC: Recenter when the cursor jumps far off screen

- Status: accepted

## Summary

When the cursor leaves the text area, `adjust_viewport` scrolls by the minimum
needed to bring it back -- one row if it moved one row out, fifty if it jumped
fifty. That is right while the destination still overlaps the screen the reader
is looking at: the reader is following a move they just made, and the least
disruption is to slide the screen by the same amount. It is wrong once the
destination and the screen share nothing: the cursor lands one row inside the
edge, with almost none of its surroundings visible, and the reader has to
scroll by hand to see where they are.

This proposal keeps the minimum scroll while the destination overlaps what is
already visible and centers instead once it does not. The threshold is a single
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
them makes the walk hard to follow. The landed change to the search steps took
the minimum scroll side of this for granted, for exactly that reason.

What makes the small slide right is not the size of the move on its own but
what the reader can still see of it: while the destination is within a screen
of the viewport, the lines around the destination are lines the reader was
looking at a moment ago. The screen keeps its meaning, and sliding it by a row
or two preserves the connection the reader is following.

The same rule misfires when the destination and the screen stop overlapping.
`C-x e` (buffer end) can move the cursor thousands of rows at once. So can a
search step to a hit far down the file and a click far from the cursor. In each
case the cursor lands on the last visible row (or the first), its surrounding
lines are just past the edge, and the reader's next action is almost always to
scroll toward what they jumped to. Nothing of the screen they left is near the
destination, so the slide buys no continuity at all: the reader gets a window
whose whole visible content is unfamiliar. Centering at the moment of the jump
would have put the destination in the middle, which maximizes the surroundings
of a position the reader did not already have in view.

So the two moves want opposite things, and whether the destination still
overlaps the screen is what tells them apart:

- a cursor within a screen of the viewport shares rows with it; the reader has
  context on screen and a small slide preserves it;
- a cursor more than a screen out shares nothing with it; the reader has no
  context, and pinning the destination to an edge shows the least of it.

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
takes the small slide, however it got there; a cursor more than a screen below
is far out and centers. Walking off the edge with `C-n` therefore keeps the
one-row slide, and so does a search step to a hit just past the last row.

A jump that lands more than a whole text area away from the viewport centers:

```text
before:                              after C-x e (jump far down):

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
let overlapped = available_rows; // one whole text area; see below

if cursor_pos.row < self.viewport.row {
    let rows_out = self.viewport.row - cursor_pos.row;
    if rows_out > overlapped {
        self.viewport.row = cursor_pos.row.saturating_sub(available_rows / 2);
    } else {
        self.viewport.row = cursor_pos.row;
    }
} else if cursor_pos.row >= self.viewport.row + available_rows {
    let rows_out = cursor_pos.row - (self.viewport.row + available_rows).saturating_sub(1);
    if rows_out > overlapped {
        self.viewport.row = cursor_pos
            .row
            .saturating_sub(available_rows.saturating_sub(available_rows / 2));
    } else {
        self.viewport.row = cursor_pos
            .row
            .saturating_sub(available_rows.saturating_sub(1));
    }
}
```

`rows_out` counts how far the cursor is *past* the edge, not how far it moved:
a cursor on the first row outside the viewport is one row out. Centering is the
same arithmetic the recenter branch already uses
(`cursor_pos.row.saturating_sub(available_rows / 2)`), recomputed against the
*new* cursor rather than the old one, so the plain-recenter and
threshold-recenter paths land on the same viewport when they fire.

### The threshold value

One whole text area: a cursor centers when it is more than `available_rows`
rows outside the viewport, and slides by the minimum otherwise. The threshold is
a property of the text area, not a fixed row count, so it keeps its meaning as
the terminal resizes, and it is a named constant beside the rule rather than a
literal at the comparison.

One screen is exactly the point where the overlap runs out, which is what the
motivation rests on. The viewport covers `available_rows` rows, so a cursor that
is `available_rows` rows out has a line that touches the edge row, and only
past that does the destination's neighbourhood -- the `available_rows` rows a
centered scroll would show, half above the cursor and half below -- share no row
with the screen being left. Nearer than that, some of what would be revealed is
also what the reader is looking at, so the slide keeps its connection.

Half a text area was the alternative, on the theory that centering only replaces
a scroll of more than half a screen and so a cursor just past the middle is
still near. That reads the amount of movement as the signal, but the movement
is not what decides the case: a cursor half a screen out is drawn from lines
half of which are still on screen, so centering it would throw away a
connection that a slide would have kept. It is also degenerate on short text
areas. With the gutter's summaries taking their rows, a three-row area leaves
one text row, so half of it is zero and *every* off-screen cursor would center
-- exactly the one-row walk through adjacent hits that the guide-level examples
call out as the behavior to keep. One screen has no such degenerate case: a
cursor one row out is one row out whatever the area's height.

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

The rule only fires on a cursor that is more than a whole screen outside the
viewport, so the tests that walk one row at a time keep their current
expectations and a new set covers the far case:

- a cursor one row outside the viewport still scrolls by one row (the walk
  case must not regress);
- a cursor more than a text area outside scrolls so it is centered;
- the boundary: exactly `available_rows` rows out stays minimal,
  `available_rows + 1` centers;
- an explicit `C-l` still centers even when the cursor is a screen out.

`tests/state.rs` has the boundary pair (`a_cursor_exactly_a_screen_out_is_not_
centered`, `a_cursor_a_screen_and_a_row_out_is_centered`), the upward case, the
explicit-recenter precedence, and a zero-height area that must not underflow.
`tests/search.rs`'s `a_step_to_a_hit_a_screen_away_centers_it` checks the far
step, and the existing hit tests keep exercising the small side: with a
three-row area and the gutter's summaries they move the cursor one row past the
edge at a time, so the threshold stays out of the way.

One earlier draft asserted the boundary with `handle_cursor_to_position` and
forgot that each call recenters before the adjustment runs; the tests set the
viewport directly instead, so the cursor move being measured is the only thing
that decided it.

## Drawbacks

- **A second scrolling mode.** Today "the cursor is visible and the viewport
  follows" is one rule with one exception (the explicit request). This adds a
  second automatic mode whose boundary is a number the reader has to feel out.
  A cursor one screen out does something different from a cursor a screen and a
  row out, and neither is obvious from the key that moved it.
- **The same move can take either branch as the terminal resizes.** The
  threshold is a screen, so a jump of a fixed number of rows slides on a tall
  terminal and centers on a short one. That is the price of keeping the rule
  meaningful at every size, and the alternative (a fixed row count) has the same
  problem in the other direction.
- **Centering on a jump can still surprise.** A reader who jumps with `C-x e`
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

- **Per-caller choice** (the search steps recenter, plain moves do not, some
  future paging command does). Rejected because it multiplies rules per command
  and gives no answer for a new command: the reader has no model to predict
  from. The search steps in particular already say "keep it visible" and should
  say it once, not encode a size-dependent decision.

- **A fixed row count instead of a fraction of the screen.** Easier to state
  ("more than 5 rows out centers") but it means different things on a 10-row and
  a 60-row terminal, and a big terminal would center for moves that are a small
  fraction of it. Rejected in favor of a fraction.

- **"Always keep the destination visible, and let the reader recenter by
  hand."** Closest to the current behavior, and the explicit `C-l` already
exists for the rest. Rejected: it puts the burden on the reader at the moment
  they are least oriented.

## Unresolved questions

- The threshold is settled: a whole text area (`rows_out > available_rows`),
  because that is where the destination's neighbourhood stops overlapping the
  screen being left (see "The threshold value"). A smaller one -- half a screen
  -- would center while the reader still has half the destination's lines on
  screen, and it degenerates on a one-text-row area.
- The distance is settled as the distance *outside the viewport*, not the
  distance the cursor moved. It is what `adjust_viewport` can see, it needs no
  last-cursor field or per-caller argument, and it treats a held `C-n` as the
  walk it is: each adjustment is one row out, never a screen.
- Columns stay on the minimum scroll (see the Horizontal section). A long line
  can put the cursor many columns out, but there is no horizontal paging and the
  large column moves that exist (`C-a`, `C-e`, a click) are line-local, where
  pinning the edge is right. The same threshold can be extended to columns later
  without a new mechanism.
- Paging-style opt-out is moot: kk has no `C-v`/`M-v`. If paging is added, the
  command itself can set `recenter_viewport` and get centering directly, so no
  intent flag has to live in the threshold.

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

## Outcome

Implemented in [#10](https://github.com/sile/kk/pull/10) (merged as `e882800`).

## Outcome

Implemented in PR #10, merged as `e882800`.

`adjust_viewport` now centers the cursor on its row when the jump leaves more
than one text area of rows outside the viewport (that is, when the cursor's
surroundings no longer overlap what was on screen). Shorter jumps keep the
minimal scroll, which is what every ordinary one-row walk still does. The
threshold needs no configuration and no state: it is a single comparison
against `available_rows` inside the existing vertical branch.

The rule is documented on `adjust_viewport` rather than as a named constant,
because "one text area" is the definition and a constant would only restate
it. Horizontal scrolling intentionally stays minimal at any distance, and `C-l`
remains the way to center on demand.

The note's open question about the distance metric was settled in favor of
"rows outside the viewport" rather than "rows moved", so no new argument or
field was needed. The candidate half-screen threshold from the draft was
dropped: at a one-row text area it degenerates to zero and would recenter on
every step, changing how fast walks scroll.

Coverage: `tests/state.rs` pins both sides of the boundary (exactly one text
area of rows out stays minimal, one row beyond centers) plus the upward jump,
`C-l`, and a zero-height guard; `tests/search.rs` shows a search hit a screen
away landing centered while the existing near hits keep scrolling minimally.

The scope is unchanged from what is described above.
