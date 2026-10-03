# RFC: C-l reaches the screen's edges at the ends of the file

- Status: open

## Summary

`C-l` cycles the cursor through the three places there are to put it -- the
middle of the text area, the first drawn row, the last drawn row -- and near the
middle of a file each place is where it says it is. Near an end of the file they
are not: the rows a place names are computed by subtracting from the cursor's
row, and the subtraction saturates at row `0`, so a `Top`/`Bottom` (and a
`Center`) asked for close enough to an end of the buffer leaves the viewport at
row `0` and the cursor somewhere short of the edge the place names. A reader who
asks for the cursor on the last drawn row gets it a few rows above instead,
because there are not enough lines above the cursor to push the frame down.

That is the wrong thing to give up. A manual `C-l` press is the reader asking
for a place on the screen, and the place should be shown whether or not the
buffer has the rows to fill the frame around it. The screen can already show
blank rows past the end of the file; showing blank rows before the start is the
same kind of thing, and it is what lets `Bottom` near the top of a file put the
cursor on the last drawn row. This proposal makes the manual places reach the
screen's edges at the ends of the file, leaving the blank rows that takes.

This is a change to the manual `C-l` alone. The automatic recenter -- the one
that fires on a jump the reader did not ask to place -- keeps its own rule, and
is the subject of the companion proposal that floors a far recenter at the last
drawn row so it does not add blank rows. The two are already split by who asked,
and this proposal keeps the split: the automatic recenter avoids blank rows, and
the manual place honors the place and takes them.

The places themselves stay a cycle; that is the existing behavior and is not
what this proposal changes.

## Background

The next place is chosen by `State::recenter_place()` in `src/state.rs`, and the
viewport row a place works out to is computed by `RecenterPlace::row()`:

```rust
fn row(self, cursor_row: usize, available_rows: usize) -> usize {
    match self {
        Self::Center => cursor_row.saturating_sub(available_rows / 2),
        Self::Top => cursor_row,
        Self::Bottom => cursor_row.saturating_sub(available_rows.saturating_sub(1)),
    }
}
```

Every subtraction saturates at `0`, so `row()` cannot name a row above the
file's first line. `Top` names the cursor's own row and is never a problem, but
`Center` and `Bottom` both subtract enough to saturate whenever the cursor is
within half a screen of the file's first line:

- `Bottom` saturates when `cursor_row < available_rows - 1`, that is, whenever
the cursor is nearer the first line than the frame is tall;
- `Center` saturates when `cursor_row < available_rows / 2`, which is every
  cursor position within half a screen of the first line.

At the far end of the file the same shape appears for `Top`: the cursor's row is
near the last line, and putting it on the first drawn row would leave the frame
mostly blank below, but nothing stops that, so `Top` near the end already shows
blank rows. The asymmetry is that blank rows *below* the text are allowed today
and blank rows *above* it are not, and it is the second kind that a `Bottom`
near the top of the file needs.

## Problem

A manual `C-l` near an end of the file does not reach the edge it names.

At the start of the file, a `Bottom` request asks for the cursor on the last
drawn row, which means a viewport row of `cursor_row - (available_rows - 1)`.
For a cursor on row `1` in a five-row area that is `1 - 4`, which saturates to
`0`, so the viewport stays on row `0` and the cursor is drawn on row `1` of the
area -- the *second* drawn row, not the last. The reader asked for the bottom
and got neither the bottom nor the top.

The same happens to a `Center` request within half a screen of the first line:
the viewport saturates at `0` and the cursor is drawn above the middle of the
area instead of in it.

The cause is one thing: the viewport row is a `usize`, the file's first line is
row `0`, and there is no representation for a frame whose first drawn row is
*above* the file's first line. Blank rows below the end of the file already
work because the viewport is a real buffer row there and the renderer draws
nothing for rows past the last line; blank rows above the start have no such
row to point at.

## Proposal

Let a manual `C-l` place the cursor on the drawn row its place names, at the
ends of the file as well as in the middle. When the place needs more rows above
the cursor than the file has, the frame's upper rows are drawn blank, mirroring
the blank rows already drawn below the end of the file, so:

- `Top` puts the cursor on the first drawn row. Near the end of the file this
  leaves blank rows below, as it does today;
- `Bottom` puts the cursor on the last drawn row. Near the start of the file
  this leaves blank rows above, which is the new part;
- `Center` puts the cursor in the middle of the drawn rows. Near either end this
  leaves blank rows on the short side.

The reader asked for a place on the screen, and the place is what is shown. The
blank rows are the cost of showing it near an end of the file, and they are the
same rows the reader already accepts below the end of the file.

Nothing about the cycle changes: the presses still visit Center, Top, Bottom and
back, and the next place is still read from the viewport the last adjustment
left. Only the row a place works out to changes, and only when the file is too
short above the cursor to fill the frame. Far from the ends of the file every
place is already where it says it is, and nothing moves.

## Design

The change is in `RecenterPlace::row()` and in the viewport it writes, and in
nothing else about `recenter_place()` or the cycle. `row()` stops saturating at
`0`: the row it names may be negative relative to the file's first line, and
that is the signal for the frame to start above the file. The viewport still
takes a `usize` for the renderer's sake, but the manual places may point before
the file's first line, and the renderer draws the rows before row `0` as blank.

How the "before row `0`" state is represented is the open question below; the
rest of the change is clear:

- `RecenterPlace::row()` returns a signed row (or an equivalent "rows above the
  file" count beside the unsigned row) so that `Bottom` near the top of a file
  can name a row before the first line;
- the draw path treats a frame whose first drawn row is above the file's first
  line the same way it treats a frame whose last drawn row is past the file's
  last line: the rows with no buffer line are left blank;
- cursor placement, hit testing, clicks, and the search highlight all already
  map between screen rows and buffer rows through the viewport; they get the
  same "row before the file" handling as the draw path, so a click on a blank
  row above the file lands nowhere rather than wrapping to a real line.

The `Top` behavior near the end of the file does not change: it already names
the cursor's row and already shows blank rows below when the file ends first.
This proposal only adds the mirror at the top.

## Open questions

- **How to represent a frame that starts above the file's first line.**
  `viewport.row` is a `usize` today, and the file's first line is row `0`; a
  frame starting one or more rows above it has no `usize` row to point at. The
  options are, at least: (a) make the field signed (or a small struct of an
  "above" count and a `usize` row), (b) keep `viewport.row` at `0` and add a
  separate "blank rows above" count that the renderer and the row/column
  mapping read, or (c) keep the frame's top at row `0` and give only the cursor
  a screen row above it, which does not work because the cursor must stay inside
  the drawn area. Each option reaches into the renderer and the screen/buffer
  row mapping; which one is least disruptive is not yet settled.

- **What the automatic recenter should do near the start of the file.** It
  already avoids blank rows by flooring at the last drawn row when the cursor is
  near the end; the companion proposal handles that end. Whether it should
  mirror that at the start -- and whether a far jump that lands near row `0`
  should center or clamp -- is left to the companion proposal, since this one is
  about the manual press.

- **Whether a `Bottom` request on the file's first line should be a no-op or a
  move.** With blank rows above allowed, `Bottom` from row `0` could show the
  cursor on the last drawn row with the rows above it blank, instead of the
  no-op it is today (the viewport cannot go above row `0`, so the press lands on
  row `0` again). Leaving it a no-op keeps the first line from moving on a press
  that names no real row above it; making it move is more consistent with the
  rest of this proposal. Not settled here.

## Alternatives

- **Leave the ends of the file as they are.** A press near an end clamps at row
  `0` and the cursor stops short of the edge. This is the current behavior, and
  it is what this proposal exists to change; it is listed to be clear that the
  blank rows are the point, not a bug to avoid.

- **Clamp at the file's first line and say so in the message.** The press could
  name the place it could not fully reach (for example, "Cursor at bottom (at
  start of file)") rather than silently landing short. This is truthful about
  the clamp but still does not put the cursor where the reader asked; it is a
  smaller change that does not meet the goal.

- **Extend the file with a virtual blank line above row `0`.** Give the buffer
  itself a leading empty line the cursor can be placed on, so the frame never
  needs to start outside the buffer. Rejected: it changes what the file is -- the
  line would be editable and saveable unless special-cased everywhere -- for a
  display need.

- **Represent the frame as "first buffer row" plus "rows of lead-in".** A pair
  of values, where the lead-in is the count of blank rows above the first
  buffer row. This is the concrete form of option (b) in the Open questions and
  is the leading candidate; it is listed here as an alternative because it is a
  bigger change to `State` than a signed row and the choice is not yet made.

## Impact

Ergonomics, with a real change under the hood. What a reader sees changes only
near the ends of a file: a manual `C-l` that used to stop short of the edge now
reaches it, and the rows between the cursor and the edge are blank. In the
middle of a file nothing changes.

Under the hood the viewport's row may point before the file's first line, so the
renderer, the cursor's screen position, hit testing, and clicks all need to treat
"above the file's first line" the way they already treat "below the file's last
line." The signed-or-paired representation from the Open questions is the whole
of the risk; the cycle and the place selection are untouched.

The companion proposal about the automatic recenter near the file end is
unaffected: it is about the rows below the cursor, and this one is about the
rows above.
