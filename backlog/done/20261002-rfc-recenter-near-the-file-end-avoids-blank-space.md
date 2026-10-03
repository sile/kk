# RFC: Recenter near the file end avoids blank space

- Status: accepted

## Summary

When the automatic recenter fires and the cursor lands near the end of the
file, it centers the cursor and leaves blank rows under the last line. The
blank rows are wasted: the file has fewer rows left below the cursor than the
screen can show, so there is nothing to put in the lower half of the frame.
The same jump toward the start of the file already avoids this, because the
centering formula saturates at row `0` and leaves the cursor at the top of the
frame. This RFC makes the end of the file behave the way the start already
does: an automatic recenter that lands near the file end puts the cursor on the
last drawn row instead of centering it.

This is also the tail of the far-jump rule
(`20260929-rfc-recenter-when-cursor-jumps-far.md`): a cursor more than a text
area outside the viewport is centered, and this RFC is what keeps that center
from spending the lower half of the frame on rows past the end of the file.
Nothing here introduces a second scrolling mode of its own.

## Background

The automatic recenter is the one in `State::adjust_viewport` in `src/state.rs`.
When the cursor is more than a whole text area outside the viewport, a jump that
shares no row with the screen being left, the viewport is centered rather than
pinned to the edge the cursor came in through:

```rust
// A cursor more than a screen out shares no row with the screen
// being left, so it is centered rather than pinned to the edge
// it came in through; nearer than that, the lines around the
// cursor are lines the reader was just looking at, and the
// minimum scroll keeps the connection.
if rows_out > available_rows {
    self.viewport.row = cursor_pos.row.saturating_sub(available_rows / 2);
}
```

This is the recenter `C-x C-e` (buffer end) and a search that jumps far away
take, along with any other command that moves the cursor more than a screen.
`C-x a` (buffer start) takes it too: it puts the cursor at row `0`, and
`0.saturating_sub(available_rows / 2)` is `0`, so the cursor comes to rest on
the first drawn row with no blank rows above it. The start of the file is
placed well not because it is special-cased but because the centering formula
cannot put the cursor anywhere else once there is nothing above it.

The end of the file has no such accident in the plain centering formula. A
cursor on the last row, or on any row with fewer than `available_rows / 2` rows
below it, still centers: the formula subtracts from the cursor row and there is
no lower bound to saturate against, so the viewport is left far enough above
the file end that the rows below the cursor run out before the frame does.
Those rows are drawn blank.

The floor that fixes it is the row the `C-l` cycle names for `Bottom`,
`cursor_row.saturating_sub(available_rows.saturating_sub(1))`, balanced against
the file's end: the automatic recenter takes the centered row, pulled up to the
bottom place when that is lower, and capped so the viewport never asks for a row
past the last line. (An earlier draft floored with `max` alone; see
[the correction](#correction-the-floor-is-a-balance-not-a-max).)

## Problem

The blank rows are not a useful placement. The screen exists to show the file;
when the file has nothing left to show below the cursor, the rows that would
have held it are better spent showing more of the lines above, and the cursor is
more useful against the bottom edge where the reader's eye is already looking
for the end of the file.

The asymmetry is the real defect. The start of the file and the end of the file
are the same situation turned upside down -- no context on one side of the
cursor -- and the current rule treats them differently for no reason other than
which side `saturating_sub` happens to guard. A reader who jumps to the start
sees the cursor against the top edge; a reader who jumps to the end sees it
halfway down with dead rows under it.

## Proposal

When the automatic recenter centers the cursor, floor the center at the row that
puts the cursor on the last drawn row, and cap it at the file's end. The floor
is the formula the `C-l` cycle's `Bottom` place already uses; the cap is where
the last drawn row reaches the last line:

```rust
let centered = cursor_pos.row.saturating_sub(available_rows / 2);
let bottom = RecenterPlace::Bottom.row(cursor_pos.row, available_rows);
let last_viewport = (self.row_count() + 1).saturating_sub(available_rows);
self.viewport.row = centered.max(bottom).min(last_viewport);
```

Expressed as a condition, the floor applies when the centering would leave the
cursor with fewer than `available_rows / 2` file rows below it:

```text
cursor_pos.row + available_rows / 2 >= rows()
```

It is a saturation, not an equality against `rows() - 1`: a cursor on the last
row satisfies it, and so does a cursor one row above the last when the frame is
short, and a search that lands a few rows before the end. Any position with too
little file below it to fill the lower half of the frame is floored. This is
the mirror of the start, where the centering formula already saturates because
there is too little file above the cursor.

The floor and the cap are two bounds, not one formula. The `Bottom` place is the
floor on how far the center is pulled back, and the file's end is the hard cap.
They are different by construction -- `cursor - (n - 1)` against `cursor - n / 2`
-- so no single expression is both, and the `max` and `min` are both needed.
`max` alone is a no-op: `bottom` is never above `centered` for `available_rows
>= 2`, so a `max`-only version leaves the center untouched and the blank rows in
place. `min` alone is the opposite error: at the file's last row it floors to a
viewport that stops a screen short of the file's end and hides the last line.
The pair has no threshold row count to get wrong.

The rule belongs to the automatic recenter only. An explicit `C-l`
(`handle_view_recenter`) still cycles through Center, Top, and Bottom and still
means what it says: a reader who asks for the center gets the center. Only the
recenter that fires on its own, on a jump the reader did not describe as a
placement, takes the new rule.

## Design

The change is one expression inside the `rows_out > available_rows` case in
`State::adjust_viewport`, next to the centering it replaces: the center pulled up
to the `Bottom` row when that is lower, then capped at the file's end. Nothing
else changes. Nothing to do with an explicit recenter changes, so `RecenterPlace`
and its three messages are untouched, and the `C-l` cycle is unchanged.

Reusing `RecenterPlace::Bottom.row()` rather than writing the arithmetic out
again keeps the floor in step with the `C-l` cycle's `Bottom`: when the floor is
the binding bound, a reader who then presses `C-l` to settle the placement sees
`Cursor at bottom` and the very spot the jump already chose. The cap is written
from `row_count()`, the same bound `handle_scroll` clamps its bottom against, so
the two views of the file's end agree.

The floor is inside the far-jump branch, so it only ever applies to a cursor
more than a text area outside the viewport -- the jump that centers in the first
place. A nearer jump keeps its minimum scroll, which is what a walk down a
screen and a step to an adjacent hit still rely on.

### Tests

`tests/state.rs`'s `an_automatic_recenter_near_the_end_leaves_no_blank_rows`
asserts that a jump near the end leaves the cursor on the area's last drawn row,
with no blank rows under it, and that a jump to the file's last row puts the last
line on the last drawn row -- the property that matters. It does not assert that
the automatic row equals the `C-l` `Bottom` row: the two are different by
construction (`cursor - n / 2` pulled to `cursor - (n - 1)` and then capped), and
they only coincide when the floor is the binding bound.

### Why not gate on the last row only

The first shape of this rule checked `cursor_pos.row == rows() - 1`. That is too
narrow. The blank rows appear whenever the file runs out below the cursor within
the frame, not only on the last row: a search that lands three rows before the
end on a frame ten rows tall centers the cursor with three rows of text under it
and four blank rows below those. Gating on the last row would fix `C-x C-e` and
leave that case, and every case like it, still wasting the lower half of the
frame. The floor-and-cap covers them all with one rule.

### Why not drop centering for every jump

Centering is still right for a jump that lands in the middle of the file: the
screen the cursor left shares no row with the destination, so the middle is the
most useful place to show it, with real lines on both sides. Only the side with
no lines left in the file needs the edge placement, and that is exactly what the
condition selects.

## Alternatives

- **Special-case `C-x C-e` alone.** The blank rows are a property of the
  placement, not of the command, so every jump that lands near the end would
  still produce them. A search that jumps near the end is the same case and
  should behave the same way.
- **Change the explicit `C-l` `Center` too.** A reader who presses `C-l` and
  stops on Center asked for the middle; moving the cursor to the bottom edge
there would ignore the request. The automatic recenter is the one the reader did
  not ask for, so it is the one that should bend to the file.
- **Always place near the file end at the bottom edge.** Equivalent to the
  proposal for the automatic recenter, and the condition above is the way to say
  "near the file end" without a magic row count.
- **Give the automatic recenter a mode of its own** (a flag a caller sets, or a
  second threshold). Rejected: the far-jump rule already decides *when* to
  center, and the blank rows are a property of *where* the center lands. One
  expression beside that rule is enough, and a mode would be a third thing to
  keep in agreement.

## Impact

Ergonomics. The file is shown at least as fully as before -- the last line is on
or above the last drawn row either way -- and a jump near the end stops spending
the lower half of the frame on rows with no text. Nothing about which rows the
file contains changes, so no correctness is at stake and no reading is affected
beyond where the cursor rests after a long jump.

## Correction: the floor is a balance, not a `max`

The proposal above first shipped as `centered.max(bottom)` in
`5f70114` ("Floor a far recenter at the bottom place's row"). That version does
nothing: `bottom = cursor - (n - 1)` is never above `centered = cursor - n / 2`
for `available_rows >= 2`, so the `max` always keeps the center and the blank
rows stay. The test written with it fitted its assertions to the regressed
behavior and its comments to the formula they were supposed to check, and the
invariant it named -- "the automatic recenter reaches the same row `C-l`
`Bottom` would" -- never held.

The shipped fix balances two bounds instead: floor the center at the `Bottom`
row, then cap at the file's end (`centered.max(bottom).min(last_viewport)`),
where `last_viewport = (row_count() + 1) - available_rows`. `max` alone under-
floors, `min` alone hides the last line at the file's end; only the pair keeps
the cursor on the last drawn row with no blank rows below it and the last line in
the frame. See
[`20261002-rfc-c-l-cycles-positions-regardless-of-the-file-end.md`](done/20261002-rfc-c-l-cycles-positions-regardless-of-the-file-end.md)
for the cycle that reads its next place from the cursor and viewport, which this
floor sits beside.

## Outcome

Implemented on `main`; no separate pull request.

`5f70114` landed the first version, which was inert (see
[Correction](#correction-the-floor-is-a-balance-not-a-max)), and `c2b7384`
landed the balanced floor that ships: `centered.max(bottom).min(last_viewport)`
in `State::adjust_viewport`. `tests/state.rs` pins that a far jump near the
file end puts the cursor on the last drawn row with no blank rows below it and
the last line still in the frame.
