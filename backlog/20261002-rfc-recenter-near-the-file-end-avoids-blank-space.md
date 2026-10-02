# RFC: Recenter near the file end avoids blank space

- Status: open

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

The end of the file has no such accident. A cursor on the last row, or on any
row with fewer than `available_rows / 2` rows below it, still centers: the
formula subtracts from the cursor row and there is no lower bound to saturate
against, so the viewport is left far enough above the file end that the rows
below the cursor run out before the frame does. Those rows are drawn blank.

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

When the automatic recenter centers the cursor and the cursor is near the file
end, place the cursor on the last drawn row instead of the middle. "Near the
file end" means there are fewer rows below the cursor than the lower half of
the frame the centering would use:

```text
cursor_pos.row + available_rows / 2 >= rows()
```

In that case the viewport row is the one that puts the cursor on the last drawn
row, which is the formula [`RecenterPlace::Bottom`] already uses:

```rust
self.viewport.row = cursor_pos.row.saturating_sub(available_rows.saturating_sub(1));
```

The condition is a saturation, not an equality against `rows() - 1`: a cursor
on the last row satisfies it, and so does a cursor one row above the last when
the frame is short, and a search that lands a few rows before the end. Any
position with too little file below it to fill the lower half of the frame is
placed the same way. This is the mirror of the start, where the centering
formula already saturates because there is too little file above the cursor.

The rule belongs to the automatic recenter only. An explicit `C-l`
(`handle_view_recenter`) still cycles through Center, Top, and Bottom and still
means what it says: a reader who asks for the center gets the center. Only the
recenter that fires on its own, on a jump the reader did not describe as a
placement, takes the new rule.

## Design

The change is one branch inside the `rows_out > available_rows` case in
`State::adjust_viewport`, next to the centering it replaces. When the condition
above holds the viewport takes the `Bottom` row for the cursor instead of the
centered one; otherwise the centered row is kept exactly as it is now. Nothing
to do with an explicit recenter changes, so `RecenterPlace` and its three
messages are untouched, and the `C-l` cycle is unchanged.

Reusing the `Bottom` formula rather than writing a second one keeps the two in
step: the row an automatic recenter picks when it lands near the file end is the
row `C-l` would pick for `Bottom`, so the reader who then presses `C-l` to settle
the placement sees `Cursor at bottom` and the very spot the jump already chose.

### Why not gate on the last row only

The first shape of this rule checked `cursor_pos.row == rows() - 1`. That is too
narrow. The blank rows appear whenever the file runs out below the cursor within
the frame, not only on the last row: a search that lands three rows before the
end on a frame ten rows tall centers the cursor with three rows of text under it
and four blank rows below those. Gating on the last row would fix `C-x C-e` and
leave that case, and every case like it, still wasting the lower half of the
frame. The saturation condition covers them all with one rule.

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

## Impact

Ergonomics. The file is shown at least as fully as before -- the last line is on
or above the last drawn row either way -- and a jump near the end stops spending
the lower half of the frame on rows with no text. Nothing about which rows the
file contains changes, so no correctness is at stake and no reading is affected
beyond where the cursor rests after a long jump.
