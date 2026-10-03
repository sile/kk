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
on a place other than the one it names, or on the page it was already showing
and move nothing. The press's message changes with it, from naming the place
asked for to naming where the cursor landed, so the line stays true.

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

Two consequences are accepted, not hidden:

- **The named place may be unreachable at an end.** `Top` near the end, asking
  for the cursor on the first drawn row, lands on the last drawn row instead,
  because the file does not have the rows below the cursor to push the frame
  down to that place. This is what the start already does to `Center` and
  `Bottom`.
- **A press at an end may move nothing.** When all three places collapse onto
  the same last page, consecutive presses can leave the viewport where it was.
  Again this is what the start already does against `0`.

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
owning it alone. The places, the cycle in `recenter_place()`, and the choice of
which branch runs are unchanged.

### Message: name the landing, not the request

Today the press's message comes from `RecenterPlace::message()`, which names the
place the reader *asked for* (`Cursor centered` / `Cursor at top` / `Cursor at
bottom`). With the cap, the request and the landing can differ -- a `Top` near
the end lands on the bottom -- so the requested-place message would report a
place the cursor is not at.

The message is replaced by a free function that names where the cursor *landed*,
read from the viewport after the height-settling loop:

```rust
fn recenter_message(cursor_screen_row: usize, available_rows: usize) -> &'static str {
    if cursor_screen_row == 0 {
        "Cursor at top"
    } else if cursor_screen_row + 1 == available_rows {
        "Cursor at bottom"
    } else {
        "Cursor centered"
    }
}
```

Called as `recenter_message(self.cursor.row - self.viewport.row, available_rows)`
when `recenter` is set, after the loop -- the landing is only known once the
viewport has settled against the summary rows. The decisions behind it:

- **The request is not a parameter.** The function sees only where the cursor
  ended up. Comparing request to landing, and so adding a note like "(top not
  reachable)", was considered and dropped: it needs the requested place, and
  carrying it to say the request failed is surface the line does not earn -- the
  reader can see the cursor did not reach the top.
- **A no-op is not its own message.** A press that moves nothing still names the
  landing. A separate "did not move" line would change what the message means
  for a case the reader can already see.
- **Order is top, then bottom, then center.** A one-row area makes `0 ==`
  `available_rows - 1`, so the top test wins there and no degenerate-height rule
  is needed.

The startup position and the automatic recenter still say nothing, as they do
today; only the manual press speaks, and it now names where the cursor ended up.

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

- **Drop the manual `C-l` message entirely.** The cursor's motion is what shows
  the move, and the automatic recenter and the startup position never speak, so
  the manual line is already the odd one out. Rejected because at the ends the
  motion can be nil (the collapse), and the line is then the only sign of where
  the press put the cursor. Recorded so the choice is visible.

- **Leave the ends as they are.** `Top`/`Center` near the end keep their blank
  rows and the message keeps naming the request. This is today's behavior; it is
  listed to be clear the blank rows are the thing being removed, not a bug to
  tolerate. It keeps the two ends asymmetric, which is the point of the change.

## Impact

Ergonomics, with one bound added and one message rewritten under the hood. What
a reader sees changes only near the ends of a file: a manual `C-l` there stops
showing blank rows, lands on the last page, and the message names where the
cursor landed rather than where it was asked to go. In the middle of a file
nothing changes -- all three places are already where they say they are, the cap
does not bite, and the landing equals the request.

No correctness is at stake: the set of rows the file contains is untouched, and
a cursor that lands on an end place is where the same press would have put it
before the cycle existed. The manual and automatic paths now share one end
bound, which removes the asymmetry that made the end behave unlike the start.

## Unresolved questions

None. The message wording is unchanged from today's (`Cursor at top` and so on,
read from the landing instead of the request), and whether the message should
mention the collapse was settled no.
