# RFC: Show a per-line hit count gutter while searching

- Status: implemented

## Summary

While a search prompt is open, add a narrow gutter to the left of the text
area. The gutter shows, on each visible buffer line, how many matches that line
holds, and above and below the visible lines, how many matches sit before and
after them. The text area shifts right by the gutter's fixed width while the
prompt is open, and returns when it closes.

The point is to make a search self-describing at a glance: the reader can see
where the hits are in the file, how many they are walking past, and how many
are left, without leaving the buffer for a list of matches.

## Motivation

kk's search is a step-through prompt: `C-s`/`C-r` walk one hit at a time, and
the status line reports progress as `🔍n/total`. That tells the reader where
they are in the *hit sequence* but nothing about where the hits are in the
*file*. Walking from the first hit to the tenth, the reader cannot tell whether
the next one is on the following line or three hundred lines down, whether the
block they are in is dense with hits or has one and then a gap, or how much of
the file is left.

The buffer itself shows only the visible slice, so it answers none of this. The
highlights mark the hits that happen to be on screen; everything outside is
invisible until the reader scrolls or steps to it.

A gutter closes that gap without a second pane. The counts next to each line
say where the hits are inside the visible slice, and the totals above and below
say how the visible slice sits in the file:

```
 2 :                       <- 2 hits above the visible lines
   |
   |
 1 | foo bar
   |
 3 | foo foo foo
 5 | baz
   |                       <- 0 hits below, so this row is omitted
```

Here the reader sees three hits on the current screen, two above, and -- since
the bottom total is omitted -- none below: the search is finished below this
point. None of that requires stepping through the hits or scrolling.

## Guide-level explanation

While a search prompt is open, the editor reserves a gutter on the left of the
text area. It is always there in that mode, even before anything is typed or
when nothing matches, so the mode is recognizable at a glance and the text does
not jump around as the query changes.

The gutter has a fixed width of five columns. A buffer line reads
`<count>| <line text>`:

```
<count>| <line text>
```

- `<count>` is right-aligned in two columns.
- A count above 99 is shown as `99+` in three columns; the exact value stops
  being useful past that, and letting the width grow would shift the text every
  time the count crossed a digit.
- A line with no hits leaves the count columns blank.
- A space follows the `|` before the line text.
- On the cursor's line, the count cell is reversed, so the gutter marks the
  cursor as plainly as the reversed line text does.

So the widest row is `99+| ` and every row lines up:

```
 1 | foo bar
23 | baz
99+| qux
   | (a line with no hits)
```

Above and below the visible lines, when there are hits outside them, the gutter
shows one summary row each, in the same fixed width, rounded with the same
`99+` when the total is past two digits. A summary row reads `<total>:`, using
`:` where a buffer line uses `|`, so a reader can tell at a glance that the row
is a total and not a line of the text:

```
 2 :                       <- 2 hits above the visible lines
   |
 1 | foo bar
 5 :                       <- 5 hits below
```

- the top row shows the total number of hits in the lines *before* the first
  visible line, or is left out when that total is zero;
- the bottom row shows the total in the lines *after* the last visible line, or
  is left out when that total is zero.

The two rows are not buffer lines: each takes a row of its own, the top total
above the first visible line and the bottom total below the last, so the text
area loses a row of height at each edge while a summary is shown. (In the
sketch above, the blank `|`-rows between them and the hits are just buffer
lines whose text is blank, drawn as they are.)

The two totals are measured against the slice the frame could hold at full
height -- that is, before any summary is drawn -- so the totals themselves do
not depend on which summaries are shown. The viewport, though, is scrolled
against the height the text is *actually drawn in* once the summaries have taken
their rows: placing it against the full height would put the cursor on a row a
summary then covers, and the cursor would vanish exactly when a hit lies at the
edge. Since that height depends on which summaries appear, which in turn depends
on the viewport, the two are settled together: the scroll is repeated until the
viewport and the height it implies agree. On a frame of a single row the top and
bottom totals would collide; the top wins, since one row cannot hold two
summaries and the nearer edge is the more useful one.

Only the rows that are actually painted are counted. A hit is counted on the
line it starts on, matching the highlight. The totals are about the lines
outside the visible slice, so they do not change as the cursor moves within the
slice, only as the viewport scrolls.

When the prompt closes, the gutter disappears and the text area returns to the
full width; nothing else about rendering changes.

## Reference-level explanation

### Where the width comes from

The gutter is a constant, not derived from the buffer: five columns, of which
two are the count, one is the `+` that may fill the third, one is the separator,
and one is the space after it. Keeping it constant is the whole point -- the
text area's usable columns must not depend on how many hits the query found.

```rust
/// Columns reserved to the left of the text area while a search prompt is
/// open: a two-column count, an overflow `+`, the separator, and a trailing
/// space.
const HIT_GUTTER_COLS: usize = 5;
```

The separator is `|` on a buffer line and `:` on a summary row, so the two kinds
of row can be told apart by shape alone. Both are otherwise the same width, and
a summary row has no text after its separator.

### What counts as a hit on a line

The count is `Highlight::count_on_row()`: the number of `HighlightItem`s in
`state.highlight` whose `start_position.row` is that line. Counting by start row
matches how `render_line` decides `is_highlighted`, so a hit that wraps is
counted once, on the row it starts. The same predicate is what
`Highlight::count_outside()` sums over the rows outside the visible slice.

### Drawing the gutter

`render_text_area` already walks the visible rows and knows `start_row`,
`end_row`, `viewport.col`, and the frame size. The gutter is drawn from that
same walk rather than a separate pass, so the two cannot disagree about which
rows are visible:

- for each visible row, compute the hit count for that row and write the count,
  the `|` separator, and a space at columns `0..HIT_GUTTER_COLS`, reversed on
  the cursor's line to match the reversed line text;
- write the line text starting at column `HIT_GUTTER_COLS` instead of `0`,
  subtracting the same constant from the width used for any column clamping;
- measure the totals against the full-height slice `[start_row, end_row)`
  first, then clip the text to the rows left after reserving one for each
  non-zero total;
- scroll the viewport against the height the text is drawn in -- the full height
  less one row per summary [`adjust_viewport()`] will draw -- repeating the
  scroll until the viewport and that height agree, so the cursor lands in a row
  the text occupies rather than one a summary covers;
- scroll it against the width the text is drawn in as well -- the area's full
  width less the gutter's columns -- so a cursor near the right edge is placed
  in a column the text is drawn in rather than one the gutter covers. Unlike the
  height this needs no repetition: the gutter is drawn whenever the prompt is
  open, so the width does not depend on where the viewport settles;
- for the top total, sum the counts of the rows before `start_row` and write it,
  when non-zero *and* `start_row` is not zero, on the frame's first row as a
  count, a `:` separator, and a space, in the gutter's columns, pushing the text
  down one row;
- for the bottom total, sum the counts of the rows at and after `end_row` and
  write it, when non-zero, the same way on the frame's last row, pulling the
  text's last row up one.

The renderer needs the query's hits, which are already on `state.highlight`, so
no new state is required to draw it. Whether the gutter is shown at all is
`state.search_prompt.is_some()`, the same test `render_status_line` uses for the
`🔍n/total` segment.

### Layout

The gutter eats into the text area's columns, so the width available to line
text is `frame.size().cols - HIT_GUTTER_COLS` (saturating, so a narrow frame
degrades to zero rather than underflowing). Horizontal scrolling still follows
`viewport.col`; the gutter is not part of the scrollable area and never scrolls
with it. When the text area is narrower than the gutter, the gutter wins and no
line text is drawn, which is the same as today's "line clipped to zero width".

### Counting cost

The gutter asks for a row's count and for the totals on either side of the
visible slice once per render, and the viewport is scrolled by repeating that
ask until it settles. Walking `Highlight::items` for each ask would make every
one of them linear in the number of hits, and there are several per frame.

`Highlight::items` is built by a forward scan of the buffer, so it is already
in row order. `Highlight` therefore keeps, beside the items, one run per row
that holds a hit and a prefix sum of their counts, both built once when the
search runs. A row's count is then the one run that names it, found by a binary
search, and a total is the difference of two prefix sums. The per-render work
becomes a logarithm per ask instead of a walk, and the search itself pays for
the runs once.

The rendering lookup `state.highlight.contains(pos)` stays as it is: it is per
visible character, but the visible slice is bounded by the screen, not the
buffer, so it is not the term that grows with the hits.

The items stay private to `Highlight` and are read through accessors, so the
runs and the items cannot drift apart.

### Tests

The gutter is pure rendering, so `tests/render.rs` is the home for it:

- a frame with the prompt open shows the count beside each visible line, `|`,
  and the line text shifted right by the gutter width;
- a count of 100 or more renders `99+`;
- a zero-hit line renders blank count columns;
- the cursor's line renders its count cell reversed, and other lines do not;
- the top total appears only when there are hits above the full-height slice,
  and likewise the bottom, and each summary row uses `:` where a buffer line
  uses `|`;
- a summary row costs a row of text height, so the visible lines shrink by one
  at each edge that shows a total;
- the top total is not drawn when the viewport is already on the first line,
  since no hit can lie above row 0;
- a jump to a hit past the edge leaves the cursor's line painted, the viewport
  scrolled so the cursor is in a row the text occupies rather than one a summary
  covers -- checked end-to-end, since the bug it fixes is only visible on screen;
- a scroll that lands the cursor at the edge keeps it inside the drawn text rows
  (a unit check on [`text_rows()`]);
- a cursor near the right edge is scrolled into the columns the text is drawn
  in, not left under the gutter -- a unit check on [`text_cols()`], since the
  columns the gutter takes are as invisible to the cursor as the rows a summary
  takes;
- on a one-row frame with totals on both sides, the top total wins;
- a total over 99 renders as `99+`, the same as a line count;
- the gutter is absent when the prompt is closed and the text starts at column
  0.

`Highlight::count_on_row()` and `Highlight::count_outside()` are the counts
themselves, so their checks live beside the search's rather than the renderer's:

- a row that holds hits counts them, and a row that holds none counts zero,
  with the rows either side of it answered correctly too (the binary search, not
  just the easy bucket);
- a slice with hits on both sides of it splits them into the right totals, and a
  slice at either end of the buffer keeps them all on one side;
- both answer zero when the query matched nothing, which is the empty-runs case
  a derived `Default` highlight also lands in.

## Drawbacks

- **The text area loses five columns while searching.** On an 80-column
  terminal that is a little under 7%, and on a narrow one it is proportionally
  worse. A reader who searches to *navigate a wide line* rather than to *find
  count* loses width exactly when they are looking.
- **A second thing to keep in sync.** The highlight rendering, the status
  line's `n/total`, the per-line counts, and the top/bottom totals are four
  views of one hit set. They can be made to agree, but there are four places
  for a future change to miss one.
- **Three-digit overflow loses information by design.** `99+` is deliberate,
  but a reader who wants the exact count of a dense region cannot get it from
  the gutter. The status line keeps the precise total, so the information is not
  gone, only not here.
- **Nothing today draws in a fixed column band across rows.** The legend and the
  status line are horizontal or corner-drawn; a gutter is the first thing that
  reserves columns for the whole height, so the renderer grows a notion of a
  left margin it did not have.

## Rationale and alternatives

- **A full results pane.** A split view listing every hit is what most editors
  do, and it answers "where are the hits" completely. Rejected for kk: it is a
  second viewport, a second cursor, and a layout rule, which is far more than
  the one question the gutter answers, and it fights the "one text, one cursor"
  shape the editor has.
- **A right-hand gutter** (counts on the right of the text). Keeps the text's
  left edge stable, but a right gutter is the side a line's continuation and
  the legend live, and the eye reads counts better against the left margin like
  line numbers.
- **A single total in the status line only** (what kk has). Cheapest, and
  already shows `n/total`, but it does not localize hits in the file, which is
  the whole motivation.
- **Variable-width counts.** Bounded only by the digits of the largest count;
  avoids the `99+` lie but makes the text area's left edge move as counts
  change, which is worse to read than a rounded number.
- **A `0` for lines with no hits** instead of a blank. Rejected: the visual
  weight of a column of zeros competes with the counts that matter, and "no
  digits" already says zero the way every line-number gutter does.
- **Tying the totals to the cursor instead of the visible slice.** Counting
  "hits before the cursor" and "after it" is a different question and would
  change under every cursor move, making the numbers flicker; the visible-slice
  totals change only on scroll, which is what makes them readable.
- **Leaving the cursor's count cell plain.** A line's count is a property of
  the line, not of the cursor, so it could stay unstyled. Rejected because the
  gutter reads as one column beside the text: a reversed line next to a plain
  count looks like the two belong to different rows, and reversing both makes
  the cursor's row read as a single band.
- **Showing the exact total instead of `99+`** for the top and bottom rows,
  since a total is the one number a reader may want precisely. Rejected for
  uniform width: the totals share the gutter's columns, so a wider total would
  shift every line below it, which is the jitter the `99+` rule exists to
  avoid. The status line still carries the exact total.
- **A toggle for the gutter**, the way the legend has one. Rejected: the legend
  is a hint and a reader may want it out of the way, but the gutter is the
  information the search was opened for, and its presence is also what makes
  the mode recognizable at a glance. Always on while the prompt is open is the
  point, not a default.
- **Marking a summary row with `...` after the count** (as the original sketch
  did) or with a direction arrow (`↑`/`↓`). Both say "not a line" too, but
  `...` collides with text a line could really hold and the arrow needs its
  meaning explained. Changing the seam instead (`:` for a summary, `|` for a
  line) needs no legend: the shape differs, so the reader sees the distinction
  without being taught it.

## Unresolved questions

None. The count width (two digits) is settled even though it is a judgment
call: searches of a hundred hits in one file are rare, `99+` says "many"
without pretending to a number, and "many" is all the reader needs from a
gutter -- the status line keeps the exact total for anyone who wants it.

## Future possibilities

The gutter is the first fixed left margin in the renderer, so if a line-number
gutter is ever wanted, the mechanism is now shared. A denser form -- a vertical
bar whose length shows relative density, a common "minimap" cue -- could sit in
or beside the same gutter.

Showing the count only while searching also leaves room for a "marks in this
file" gutter later, which is the same shape: per-line state summarized in a
fixed left column.
