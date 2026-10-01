# Bug: The hit-count gutter loses the hits a summary row displaces

- Status: fixed

## Summary

While a search prompt is open, the gutter totals the hits above and below the
visible slice over the slice of `available_rows` rows starting at the viewport.
But the text is drawn over a shorter slice -- `available_rows` less one row per
summary row -- so the rows that the summaries take fall in a gap: they are not
drawn next to a count (no `|` line), and they are not on the far side of either
total either. The hits on those rows are counted nowhere, so the gutter's
numbers do not add up to the search's own total and a hit row vanishes without a
trace while a summary is on screen.

```text
buffer: x / x / plain / plain / x / plain / plain / x   (8 rows, a hit on row 4)
frame:  4 rows, viewport row 2, query `x`

observed:              expected:
 2 :                    2 :
   |                      |   (row 2)
   |                      |   (row 3)
 1 :                      | x (row 4 -- the displaced hit)
                       1 :
```

Four hits exist; the gutter accounts for three. Row 4 held a hit and was inside
the slice the totals were measured over, but it is neither the line drawn beside
a count nor beyond the bottom total.

## Reproduction

A four-row frame, a viewport on row 2, and a hit on each of rows 0, 1, 4, and 7.
Row 4 is inside the raw slice `2..6` but is squeezed out of the drawn slice once
both summary rows take theirs.

```text
key:  C-s                       (the prompt opens)
type: x
key:  Esc                       (the legend, not needed -- any repaint shows it)

buffer:   "x\nx\nplain\nplain\nx\nplain\nplain\nx\n"
viewport: row 2, frame 4 rows x 10 cols

observed gutter rows 0..4:
  row 0: " 2 : "    (two hits above)
  row 1: "   | "    (row 2, no hit)
  row 2: "   | "    (row 3, no hit)
  row 3: " 1 : "    (one hit below)
expected: one of the drawn rows is row 4's `| x`, and the totals still read
          2 above and 1 below -- the counts sum to the 4 hits the search knows
```

The same thing is visible through the public API without a terminal:

```rust
let mut state = kk::State::new(kk::TextBuffer::new("x\nx\nplain\nplain\nx\nplain\nplain\nx\n"));
state.search_prompt = Some(kk::SearchPrompt::new());
state.handle_char_insert('x');
assert_eq!(state.highlight.len(), 4);            // passes: four hits
state.viewport = kk::TextPosition { row: 2, col: 0 };
// The totals the renderer draws: 2 above, 1 below -- 3 of the 4 hits.
assert_eq!(state.hits_outside(2, 6), (2, 1));    // passes; nothing names row 4
```

## Observed behavior

`render_text_area` in `src/render.rs` draws the two totals over the full-height
slice and the text over the shorter one, and the two slices are not the same:

```rust
let full_end_row = (start_row + available_rows).min(state.buffer.rows());
let (above, below) = if gutter_shown {
    state.hits_outside(start_row, full_end_row)
} else {
    (0, 0)
};

let text_height = state.text_rows(available_rows);
let end_row = (start_row + text_height).min(state.buffer.rows());
```

`above` counts hits before `start_row` and `below` counts hits at or after
`start_row + available_rows`. The drawn lines are `start_row..start_row +
text_height`, and `text_height` is `available_rows` less the summary rows. So the
hits on rows in `start_row + text_height .. start_row + available_rows` -- one
row per summary drawn -- are past the last drawn line and before the bottom
total, and nothing counts them.

The doc comment on `State::summary_rows` states the intent that is violated:
"The two totals are measured the same way here as they are when the gutter is
drawn: over the slice of `available_rows` rows starting at the viewport. This is
the one place that decides, so `text_rows()` here and `render_text_area()`
cannot disagree about how many rows the text is drawn in." The totals and the
drawn height are deliberately kept in step, but the *slice* the totals are
measured over was never shortened to match the drawn slice, so a two-row
summary shrinks the text without moving either total inward.

The total on the status line is unaffected -- it is `highlight.len()`, computed
independently -- which is what makes the gap visible: the gutter says the reader
is past three of four hits when the bottom total is on screen.

## Expected behavior

The summary totals and the drawn lines must partition one slice, with no row
between the last drawn line and the bottom total. The totals should be measured
over the drawn slice -- the same rows the loop walks -- so a hit on a displaced
row falls to the bottom total and is counted exactly once, which is the property
the gutter exists to give the reader. (The alternative, drawing the text over
the full slice and letting the summaries overlay the edge rows, also removes the
gap but changes what "visible" means, so it is a larger change.)

The reader-visible invariant is the one the RFC for the gutter states: the
gutter shows "how many hits each visible line holds and ... how many sit outside
the visible slice." A row that is inside the measured slice but outside the
drawn one breaks that -- it is neither drawn beside its count nor totalled.

## Impact

A correctness problem in the gutter, reachable from the public renderer: the
per-line counts and the two totals can sum to fewer than the hits the search
found, and the specific hit that is lost is invisible while both summaries are
on screen. It bites exactly when a summary is present, which is the common case
once the reader scrolls away from any hit -- the whole point of the feature.
The displaced rows are the ones nearest each visible edge, i.e. the ones a
reader is most likely to be walking toward. No resource effect; the counts are
just under-stated.

## Notes

The status line and the summaries read the same hit set, so the fix has to
change where the boundary between "above", "drawn", and "below" falls, not
which hits exist. The line counts already come from `count_hits_on_row` per
drawn row, so they need no change; it is the two totals that must be measured
against the drawn slice rather than the full-height one.

The wrinkle is that the boundary and the number of summary rows are mutually
dependent, so it is not a plain one-line swap of the end row. `summary_rows`
decides how many summaries to draw by measuring over the full slice
(`start_row + available_rows`), and `text_rows` is `available_rows` minus that
-- so the drawn slice is defined in terms of the totals that were measured over
a different slice. Moving `full_end_row` to `start_row + text_height` would make
the totals depend on `text_height`, which depends on `summary_rows`, which is
measured over the very slice being changed. The loop in `adjust_viewport` that
already settles the two against each other is where that fixed point is computed
for the viewport; the renderer needs the matching computation rather than a
second independent one, or the two can drift apart again.

Settling it turned out to need no loop after all, and the resolution is worth
recording. `text_rows()` owns the drawn height, and `summary_rows()` is now
derived from it (`available_rows - text_rows()`), so there is one definition
rather than two. `text_rows()` walks a few passes from the full height: it
measures the totals over the current slice, subtracts the summaries they call
for, and repeats. The walk terminates because the total above depends only on
the viewport (no hit can lie before the first drawn row), so only the bottom
total can change, and shortening the slice can only move hits from inside it to
below it -- the bottom total never disappears once it appears, so the size
settles after at most one shrink. Three passes are enough for every case the
property test reaches.

A regression test should pin that the per-line counts plus the two totals equal
`state.highlight.len()` whenever the gutter is drawn -- that identity holds
today for a frame with no summary row and fails as soon as one is shown, which
is why `the_gutter_totals_the_hits_above_and_below_the_visible_slice` in
`tests/render.rs` passes despite the gap (its rows 4 and 5 happen to hold no
hits).
