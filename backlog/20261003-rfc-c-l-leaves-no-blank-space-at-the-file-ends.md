# RFC: C-l leaves no blank space at the file ends

- Status: draft

## Summary

A manual `C-l` near an end of the file leaves blank rows on the side with no
file. Near the start this is already avoided: `RecenterPlace::row()` subtracts
from the cursor's row with `saturating_sub`, which holds the viewport at row `0`,
so `Top`/`Center`/`Bottom` near the first line stop at `0` and nothing blank is
drawn above it. Near the end there is no matching bound, so `Top` and `Center`
place the viewport too low and blank rows are drawn under the last line; only
`Bottom` lands cleanly.

This RFC gives the end the bound the start already has: cap every manual place
at the last page, `last_viewport = (row_count() + 1) - available_rows`, the same
bound the automatic recenter already uses. A manual `C-l` near an end then shows
no blank rows, mirroring the start. The cost is stated up front and accepted:
because the places collapse onto the same page at an end, a press there may land
on a place other than the one it names. That collapse would also strand the
cycle -- if the chosen place's capped row equals the page already showing, a
press would move nothing and the reader could never reach the places that do
land -- so the cycle is walked over the capped rows, skipping any place that
collapses onto the current page (see `### The cycle at the ends`). The manual
press's message is dropped: with the places
collapsing at the ends the line would either be redundant (the ordinary move,
visible on screen) or false (naming a place the cursor did not reach), and a line
kept only for the case it gets wrong is worse than no line.

## Background

The next place in the manual cycle is chosen by `State::recenter_place()` in
`src/state.rs`, and the viewport row a place works out to by
`RecenterPlace::row()`:

```rust
fn row(self, cursor_row: usize, available_rows: usize) -> usize {
    match self {
        Self::Center => cursor_row.saturating_sub(available_rows / 2),
        Self::Top => cursor_row,
        Self::Bottom => cursor_row.saturating_sub(available_rows.saturating_sub(1)),
    }
}
```

Every subtraction saturates at `0`. That is the whole of the start's behavior:
the file's first line is row `0`, so a place that would point above it is held
at `0` and the cursor stops short of the edge it names, with nothing blank drawn
above. There is no such bound at the end -- nothing stops `place.row()` from
naming a row past the last page -- so the end is the end's mirror only in the
formula, not in the result.

Measured for a 20-row file drawn in a 5-row area (viewport row each place lands
on):

| cursor | Center | Top | Bottom |
| ------ | ------ | --- | ------ |
| `1` (near start) | `0` | `1` | `0` |
| `19` (near end) | `17` | `19` | `15` |

Near the start all three sit at or near `0` with no blank rows above; near the
end `Center` at `17` and `Top` at `19` both draw the last line above the bottom
of the area, leaving blank rows under it. `Bottom` at `15` is the last page,
`(row_count() + 1) - available_rows = 20 - 5`, and is the only place that lands
cleanly.

## Problem

A manual `C-l` near the end of the file draws blank rows below the last line.
The reader asked for a place on the screen and got a screen half empty under the
file, the same waste the automatic recenter was changed to avoid
(`20261002-rfc-recenter-near-the-file-end-avoids-blank-space.md`). The start of
the file does not do this, so the same press behaves differently at the two
ends for no reason but which side has a bound.

## Proposal

Cap every manual place at the last page, exactly as the automatic recenter
already caps its center:

```text
last_viewport = (row_count() + 1).saturating_sub(available_rows)
viewport.row  = place.row(cursor_row, available_rows).min(last_viewport)
```

The start needs no separate bound; `place.row()`'s `saturating_sub` already
holds it at `0`. With the cap, a manual `C-l` within a page of the file end
lands on the last page and shows no blank rows below the file, the way a press
near the start already shows none above it.

One consequence is accepted, and one mechanism is added so the cap does not
strand the cycle -- both stated here rather than hidden:

- **The named place may be unreachable at an end.** `Top` near the end, asking
  for the cursor on the first drawn row, lands on the last drawn row instead,
  because the file does not have the rows below the cursor to push the frame
  down to that place. This is what the start already does to `Center` and
  `Bottom`.
- **The cycle is walked over the capped rows.** A place whose capped row would
  leave the viewport where it is is skipped, and the next place in the cycle is
  tried, so a press at an end still moves to the next place that has room to
  land. Near the end only `Bottom` and the capped `Center` differ, so the cycle
  becomes a two-way oscillation between them rather than a press that does
  nothing. This goes beyond capping the place's row; the design below says how.

The cap removes only the blank rows a place *asks for* by naming a page below the
last one. When the file is shorter than the text area --
`row_count() + 1 <= available_rows` -- there is no page below row `0` to move
down to, `last_viewport` saturates to `0`, and the rows under the last line are
the file simply not filling the screen. Those rows stay: they are not a place
reaching past the file, they are the file being shorter than the area, and every
editor shows them. The cap is about the case where the file has the rows and the
place dips below the last page; it says nothing about a file that ends on
screen.

## Design

The change is one bound in `State::adjust_viewport`, beside the automatic
recenter's own. Today the automatic path already computes
`last_viewport = (row_count() + 1).saturating_sub(available_rows)` and clamps
with `centered.max(bottom).min(last_viewport)`. The manual path is the branch
that takes `place.row()` without a cap:

```rust
// automatic
self.viewport.row = if automatic {
    let last_viewport = (self.row_count() + 1).saturating_sub(available_rows);
    row.max(0).min(last_viewport)
} else {
    row // manual: no cap today
};
```

The change is to cap the manual branch with the same bound -- `row.min(last_viewport)
` -- so the two branches share one end bound instead of the automatic path
owning it alone. The places and the choice of which branch runs are unchanged.

The cycle in `recenter_place()` is changed, because capping the place's row alone
turns the collapse at an end into a press that moves nothing: if the chosen
place's capped row equals the page already showing, that press is a fixed point
and the reader cannot reach the other places by pressing again. The cycle is
therefore advanced over the capped rows instead of the uncapped ones.

### The cycle at the ends

`recenter_place()` no longer picks the next place from the current place alone.
It works out the next place each call from the cursor and the viewport: it looks
at which of `Top`/`Center`/`Bottom` the current viewport is on, starts at the
place *after* it in the cycle order center, top, bottom, and takes the first
place whose *capped* row differs from the current viewport. A place that a
previous draft would have chosen but that the cap collapses onto the current
page is stepped over, and the next candidate is tried. When every candidate
collapses onto the current page the press moves nothing, which can only happen
when the file is shorter than the area and there is no other page to reach.

The comparison is on the capped value, so it agrees with where the viewport
actually lands, and the chosen place is fixed before the height-settling loop
rather than re-chosen inside it -- otherwise a press near an end would jump
between places as the loop re-evaluated the cursor's row. The enum that names
the places is kept, but it no longer carries the cycle's state; the state is the
cursor and viewport the reader sees.

### Message: drop it

Today the press's message comes from `RecenterPlace::message()`, which names the
place the reader *asked for* (`Cursor centered` / `Cursor at top` / `Cursor at
bottom`). With the cap, the request and the landing can differ -- a `Top` near
the end lands on the bottom -- so a request-naming line reports a place the
cursor is not at.

The line is dropped rather than corrected. `RecenterPlace::message()` is removed,
and `handle_view_recenter` no longer arms any message for the press; startup and
the automatic recenter never spoke and are unchanged.

The reason is that the line's only honest use is the case it exists to get
wrong. On an ordinary press the cursor moves to the place, and the reader sees
that on screen -- the line repeats what is already visible. The line would earn
its place only when the cap sends the cursor somewhere other than the named place
or leaves it where it was, and those are exactly the presses for which no short
line is both true and useful. Naming the landing instead was tried first: it
keeps every line true, but it speaks on the common move (redundant) and stays
silent or says something obvious on the odd one, so it trades one fault for
another. A line kept only for the case it cannot help with is worse than no
line, so the line goes.

This also settles the question the earlier draft left open (whether to keep a
landing-naming message): no line is kept. A future change that wants to speak can
do so deliberately -- see Future possibilities.

## Alternatives

- **Let a manual `C-l` reach the named place at an end by allowing blank rows
  above row `0` (a signed viewport).** This is the mirror of this RFC: keep the
  named place and take the blank rows on the empty side. A prototype was built
  and measured (see
  [`20261002-rfc-c-l-cycles-positions-regardless-of-the-file-end.md`](done/20261002-rfc-c-l-cycles-positions-regardless-of-the-file-end.md),
  `### Prototype`). It reached the three distinct places at an end, but the
  signed `viewport.row` and the unsigned cursor disagree about "inside", the
  wheel moves oddly just after a top recenter, and about three dozen assertions
  need the signed type. The representation cost is paid once but is felt by
  every later feature, so the direction was dropped. This RFC is the other arm
  of the same mirror, and the smaller one.

- **Keep the message, naming where the cursor landed.** A free function reads
  the viewport after the height-settling loop and says `Cursor at top` /
  `Cursor at bottom` / `Cursor centered` from the landing (`cursor.row -
  viewport.row`), so no line ever names a place the cursor is not at. This was
  the draft's proposal and is now rejected: it keeps the line true on every
  press, but the common press (the cursor moving to the place it named) is the
  one where the line only repeats the screen, and the odd press (the cap moving
  the cursor elsewhere) is the one where a landing line says little that the
  cursor's position does not already show. Paying a function and a call site for
  a line whose honest use is the case it cannot make clearer is not worth it.
  The name-the-request line it replaced was worse still, since it reports a
  place the cursor is not at once the cap bites.

- **Leave the ends as they are.** `Top`/`Center` near the end keep their blank
  rows and the message keeps naming the request. This is today's behavior; it is
  listed to be clear the blank rows are the thing being removed, not a bug to
  tolerate. It keeps the two ends asymmetric, which is the point of the change.

## Impact

Ergonomics, with one bound added, the cycle advanced over the capped rows, and
one message removed. What a reader sees changes only near the ends of a file: a
manual `C-l` there stops showing blank rows and lands on the last page, a press
there that would have landed on the page already showing now advances to the
next place that has room, and the manual press no longer prints a line. In the
middle of a file the motion is unchanged -- all three places are already where
they say they are and the cap does not bite -- and the only change is the missing
line, which the cursor's motion already told the reader.

No correctness is at stake: the set of rows the file contains is untouched, and
a cursor that lands on an end place is where the same press would have put it
before the cycle existed. The manual and automatic paths now share one end
bound, which removes the asymmetry that made the end behave unlike the start.

## Future possibilities

A later change could give `C-l` a line again, but only where a line carries its
weight -- e.g. when the cap sends the cursor somewhere other than the named
place, or when the file is short enough that the press cannot move at all. Any
such line would have to be true on its own, without the screen behind it, and
would replace the dropped request-naming line rather than sit beside it.

## Unresolved questions

None. The message is dropped rather than corrected: the request-naming line goes
because the cap makes it false at the ends, and a landing-naming line goes
because its only useful press is the one it cannot improve. Whether to speak
again is left to a future change (see Future possibilities).
