# RFC: C-l cycles positions regardless of the file end

- Status: open

## Summary

`C-l` cycles the cursor through Center, Top, and Bottom, but which place the
next press picks depends on where the cursor sits in the buffer. Near the end
of the file two of the places collapse onto the same viewport row, so the cycle
skips a place and the reader cannot tell what the next press will do without
knowing both the cursor's position and the file's length. A command the reader
asks for by hand should be predictable: each press should advance to the next
place, Center to Top to Bottom and back. Blank rows above or below the cursor at
the file end are an acceptable price for that, because the reader asked for the
place and the place is what should be shown.

This splits the two recenters by what they are for. The automatic recenter
should avoid adding wasteful blank rows, and is the subject of the companion
proposal to place a jump near the file end on the last drawn row. The manual
`C-l` should be easy to predict, and that outweighs saving the blank rows.

## Background

The cycle is read off the viewport by `State::recenter_place()` in
`src/state.rs`. Each press asks where the viewport is now and returns the next
place:

```rust
fn recenter_place(&self, available_rows: usize) -> RecenterPlace {
    let cursor_row = self.cursor.row;
    let center_row = RecenterPlace::Center.row(cursor_row, available_rows);
    let top_row = RecenterPlace::Top.row(cursor_row, available_rows);

    if self.viewport.row == top_row {
        RecenterPlace::Bottom
    } else if self.viewport.row == center_row {
        RecenterPlace::Top
    } else {
        RecenterPlace::Center
    }
}
```

The three places are rows computed from the cursor: `Center` is
`cursor_row.saturating_sub(available_rows / 2)`, `Top` is `cursor_row`, and
`Bottom` is `cursor_row.saturating_sub(available_rows.saturating_sub(1))`.

## Problem

`Center` and `Top` collapse onto the same row when the cursor is too close to
the start of the file for the centering to move up at all. At row `0`,
`Center` is `0.saturating_sub(available_rows / 2)` and `Top` is `0`: the same
row. The comparison in `recenter_place` then takes the first branch,
`viewport.row == top_row`, and answers `Bottom`, so the cycle runs Center,
Bottom, Top, Center instead of Center, Top, Bottom, Center. The same collapse
happens for any cursor row where `cursor_row - available_rows / 2` saturates to
`cursor_row`.

That is why the result depends on the buffer. The reader presses `C-l` from the
middle of a file and gets the three places in order; from near the top the same
presses give a different sequence, because the rows `Center` and `Top` would
name are the same row there. The cycle is defined by the positions in the
buffer, not by the presses, so the same gesture does different things in
different places.

## Proposal

Make `C-l` cycle the three places in a fixed order, Center, Top, Bottom, Center,
no matter where the cursor is or how long the file is. Read the place the
viewport is at from *the place that was asked for last*, not from the cursor's
row, so that two places that happen to share a row are still distinct steps. A
press that asks for Top when the viewport is already on the row Top names still
counts and still advances to Bottom.

The places themselves do not change. When Top is asked for near the end of the
file, the cursor goes to the first drawn row even though that leaves blank rows
below it; when Bottom is asked for near the start, the cursor goes to the last
drawn row even though that leaves blank rows above it. The reader asked for the
place, so the place is what is shown. A press that cannot change the view -- a
text area one row tall, where all three places are the same row -- is still a
no-op, as it is now.

## Design

The current cycle remembers nothing: it reads the place back from the viewport
so that a search step or a cut between two presses continues from what is on
screen (see the rustdoc on `recenter_place`). Making the cycle position-
independent means remembering the place the last press asked for, for example a
`last_recenter: Option<RecenterPlace>` on `State` that
`handle_view_recenter` advances and `adjust_viewport` clears once the request is
applied. A move of the cursor between presses is what the old rule was trying to
follow, and it can keep following it by clearing the memory the same way
`recenter_viewport` is cleared today, so the first press after a move starts at
the appropriate place again.

Which place a first press starts at is unchanged: Center, as it is now. The
change is only in how the next place is chosen from the previous one.

### Why the memory is not a step backwards

An earlier proposal rejected remembering the place, because reading it off the
viewport keeps the cycle honest about what the reader is looking at. That is
right when the places are distinct, and it is exactly what breaks when they are
not: a reader looking at a row that is both Center and Top cannot have the view
tell them which one they asked for, because the view is the same either way. The
memory is the smallest thing that can tell the two apart, and it is cleared on
any move so the honesty the old rule wanted is kept where it matters.

### Why the file end is not special-cased

The blank rows at the file end are a consequence of the places being fixed, not
something to guard against. Guarding against them is what makes the cycle
position-dependent in the first place: a press that would center the cursor near
the end is pushed to the bottom edge, which is the Bottom place, which the next
press then misreads. Fixing the places lets the push go away, and the automatic
recenter, which the reader did not ask for, is left to be the one that avoids
blank rows.

## Alternatives

- **Keep reading the place off the viewport and skip the collapsed one.** The
  cycle would stay position-dependent, which is the problem. The reader would
  still need to know where they are to guess what comes next.
- **Give the automatic recenter the same fixed cycle.** The automatic recenter
  fires on a jump the reader did not ask to place; its job is to show the
  destination, not to honor a place, so it should avoid blank rows. The two are
  already split by who asked, and this proposal keeps that split.
- **Make `Center` and `Top` never share a row.** They share one whenever there
  are fewer than `available_rows / 2` rows above the cursor, which is the
  ordinary case at the top of a file; avoiding it would mean the center is not
  the center. The collapse is in the positions, not in the reading of them, so
the fix belongs in the cycle.

## Impact

Ergonomics. The places shown are the same places as before, so no text is
hidden or revealed differently; what changes is that the same presses in the
same order do the same thing wherever the cursor is. The cost is that `C-l` can
now leave blank rows at the file end, which the reader asked for by pressing it.
