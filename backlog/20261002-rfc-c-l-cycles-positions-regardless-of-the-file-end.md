# RFC: C-l chooses its next position from the cursor and the viewport

- Status: open

## Summary

`C-l` moves the cursor through the three places there are to put it -- the
middle of the text area, the first drawn row, the last drawn row -- but which
place the next press picks cannot always be told from the screen. Near the file
end two of the places are the same viewport row, and the code tells the places
apart by the row the viewport is on, so the cycle skips a place there and the
reader cannot predict a press without knowing where the cursor sits in the
buffer and how long the file is. A command the reader asks for by hand should
be predictable from what is on screen, and it can be: the relation between the
cursor's drawn row and the viewport's row names the next place on its own, with
nothing to remember and no two places ever being confused for each other.

What matters is not that the presses visit the places in some particular order,
but that each press is easy to call. Reading the next place off the relation
between the cursor and the viewport makes every press answerable before it is
made: the reader sees the cursor against the top edge and knows the press will
move it to the bottom, or sees it lower down and knows the press will put it
against the top. The order the three come in follows from that, and is
Center, Top, Bottom, Center for a cursor in the middle of a file; it is not a
fixed cycle the code walks, and it does not need to be.

This splits the two recenters by what they are for. The automatic recenter
should avoid adding wasteful blank rows, and is the subject of the companion
proposal to place a jump near the file end on the last drawn row. The manual
`C-l` should be easy to predict, which the relation gives it, and that
outweighs saving the blank rows a place may leave.

## Background

The next place is chosen by `State::recenter_place()` in `src/state.rs`. Each
press asks where the viewport is now and returns the next place:

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
`Bottom` is `cursor_row.saturating_sub(available_rows.saturating_sub(1))`. The
method tells them apart by which of those rows the viewport sits on, and it
needs the text area's height to compute the rows to compare against.

## Problem

The three places are rows computed from the cursor, and two of them collapse
onto the same row whenever the cursor is close enough to an edge of the file
for the other to saturate against it. `Center` and `Top` are the same row when
fewer than `available_rows / 2` rows lie above the cursor: at row `0`, `Center`
is `0.saturating_sub(available_rows / 2)` and `Top` is `0`. `Center` and
`Bottom` are the same row, and can be past the file's end, when fewer than
`available_rows / 2` rows lie below the cursor.

The test in `recenter_place` is `viewport.row == top_row` first, then
`viewport.row == center_row`. When two of the rows are equal, the first test
that matches wins and the other place is never reached, so the press that
should have asked for one of them asks for something else. At the top of a file
the rows `Center` and `Top` name are equal, the first test matches, and the
press that should have gone to the top goes to the bottom instead: the cycle
runs Center, Bottom, Top, Center, and a reader who wanted to nudge the cursor
to the top of the screen gets the bottom of it.

That is why the result depends on the buffer. From the middle of a file the two
rows differ and the presses come out in the order the code's two tests happen
to spell; near the top the same presses give a different order, because the
rows are the same there. The cycle is defined by the rows the places name in
this buffer rather than by what the reader sees, so the same gesture does
different things in different places.

## Proposal

Choose the next place from the relation between the cursor's drawn row and the
viewport's row, and from nothing else:

- the viewport's row is above the cursor -- the cursor is somewhere down the
  drawn rows -- so the next place is `Top`, which puts the cursor on the first
  drawn row;
- the viewport's row is the cursor's row -- the cursor is already on the first
  drawn row -- so the next place is `Bottom`, which puts it on the last drawn
  row;
- the viewport's row is below the cursor, which a settled viewport does not
  leave but a pending request can, so the next place is `Center`.

The relation is three states with three different places, so no two places are
ever confused for each other and no press needs anything remembered to be
predicted. A reader looking at the cursor on the first drawn row knows the next
press takes it to the last; a reader looking at it lower down knows the press
takes it to the first.

The order the places come in is Center, Top, Bottom, Center, but it is an
outcome of the rule rather than a cycle the code walks: from `Center` the
viewport ends up above the cursor, from `Top` on the cursor's row, and from
`Bottom` above it again, so the presses come out in that order without any
place being told apart by remembering it. What matters is the rule, and the
order falls out of it.

The places themselves do not change. When `Top` is asked for near the end of
the file, the cursor goes to the first drawn row even though that leaves blank
rows below it; when `Bottom` is asked for near the start, the cursor goes to the
last drawn row even though that leaves blank rows above it. The reader asked for
the place, so the place is what is shown. A text area one row tall puts all
three places on the same row and the press does nothing, as it does now.

## Design

The change is `State::recenter_place()` and its rustdoc. It currently compares
`self.viewport.row` against the rows `Center` and `Top` would name, in that
order. The comparison is what ties the places together when they share a row,
so it goes: the method reads `self.viewport.row` against `self.cursor.row`
instead, and needs neither `RecenterPlace::row()` for the place it is choosing
nor the `available_rows` it was passed.

```rust
fn recenter_place(&self) -> RecenterPlace {
    match self.viewport.row.cmp(&self.cursor.row) {
        Ordering::Less => RecenterPlace::Top,
        Ordering::Equal => RecenterPlace::Bottom,
        Ordering::Greater => RecenterPlace::Center,
    }
}
```

`available_rows` goes with it, so `adjust_viewport` calls the method without a
height and the place's row is still worked out inside the height-settling loop,
against the drawn height each pass leaves -- the part that a summary row
appearing or going away would otherwise get wrong is untouched.

Nothing is stored. There is no field on `State` for the last place, no clearing
rule to get right, and no way for a remembered place and the screen to
disagree. The state the reader cannot see -- which press of the cycle this is --
is exactly what the current code is forced to guess at, and dropping the fixed
cycle is what lets it stop guessing.

### Why the file end is not special-cased

The blank rows a place may leave are a consequence of the places being fixed,
not something to guard against. The rule reads the relation between the cursor
and the viewport, which a blank row does not change: a `Bottom` request with the
cursor near the file end still puts the cursor on the last drawn row, and the
next press reads the cursor above the viewport and asks for `Top`. The places
and the blank rows they may leave are the same as today; only the choice of the
next one is.

### One-row areas

A text area one row tall puts `Center`, `Top`, and `Bottom` on the same row:
`cursor_row - 0`, `cursor_row`, and `cursor_row - 0`. The relation still
decides -- viewport equal to cursor asks for `Bottom`, which lands on the same
row -- so the press is a no-op, as it is today, and both the relation and the
place agree that there is nowhere else to draw the cursor.

## Alternatives

- **Remember the place the last press asked for** (a `last_recenter` field on
  `State`, advanced as the request is applied). This fixes the collapse by
  telling the two places apart with a value the reader cannot see, and it needs
  a rule for when the value is cleared so a cursor move between presses does
  not continue a cycle that is no longer on screen. The relation between the
  cursor and the viewport already tells the places apart -- they share a row,
  they do not share a *relation* -- so the field and its clearing rule buy
  nothing.
- **Keep the two comparisons and test `center_row` first.** The cycle would
  then run Center, Top, Bottom from the top of a file and something else from
  the middle, which is the same position-dependence read the other way round.
  Reordering the tests moves which buffer the bug shows up in; it does not
  remove it.
- **Give the automatic recenter the same rule.** The automatic recenter fires
  on a jump the reader did not ask to place; its job is to show the
  destination, not to honor a place, so it should avoid blank rows. The two are
  already split by who asked, and this proposal keeps that split.
- **Make `Center` and `Top` never share a row.** They share one whenever there
  are fewer than `available_rows / 2` rows above the cursor, which is the
  ordinary case at the top of a file; avoiding it would mean the center is not
  the center. The collapse is in the positions, not in the reading of them, so
  the fix belongs in the choice of the next place.

## Impact

Ergonomics. The places shown are the same places as before, so no text is
hidden or revealed differently, and the press that asks for one still puts the
cursor there. What changes is that the press after it is chosen from the screen
the reader is looking at rather than from the rows the places name in this
buffer, so the same presses do the same thing wherever the cursor is. The cost
is that `C-l` can visit `Bottom` twice in a row when there is nowhere else for
the cursor to be drawn: a text area one row tall puts all three places on the
cursor's own row, so `Bottom` leaves the viewport equal to the cursor and the
relation asks for `Bottom` again. That is the one case where a press cannot
change the screen, and it is the same no-op the current code leaves there.
