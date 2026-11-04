# RFC: Let the cursor reach the row after the last line again

- Status: draft (decided: `--tail` and `C-x C-e` both move to the end row)

## Summary

The cursor is confined to the buffer's real lines, so it cannot be placed on
the empty row just below the last line. That was made a bug and closed in
[#14](https://github.com/sile/kk/pull/14) (see
`done/20261002-bug-cursor-reaches-the-row-after-the-last-line.md`). This
proposal reverses that: an editor with a file open should let the cursor stand
on the row after the last line, the way it already stands one column past the
last character. `--tail` follows, opening at that row rather than at the end of
the last line.

## Motivation

The bug was real on its own terms: `TextBuffer::rows()` is documented as "the
number of lines" and `line(rows())` returns `None`, so a cursor at `rows()`
was on a row no line backs. Undoing it is not free of that tension, but three
things make the current bound the more awkward of the two choices.

**An empty file.** An empty file has no lines at all: `rows()` is `0`, and
`last_row()` saturates to `0` so the cursor has a row to stand on. The cursor
there has a blank row below it on screen and no line behind either row. Today
that is a file whose only addressable row is one no line backs. Allowing the row
after the last line makes the empty file ordinary too: its row `0` is the row a
cursor starts on and row `1` is the row after it, the same shape every other
file has -- a whole file with no content is simply the case where both rows are
blank.

**The end of a line.** The cursor already reaches one column past the last
character -- the end-of-line position where there is no character, drawn as a
reversed blank while a search is open (`render_line` in `src/render.rs`). That
is a position that does not correspond to an existing column, and it is
accepted and used. A row after the last line is the same kind of place: a
position the buffer has no cell for, used because a cursor has to be able to
stand somewhere past the content, not because the content is there.

**Convenience.** Appending to a file is common. Today `C-x C-e` (to the last
line) then `End` (to its end) is the shortest way to get to"after the last
line", and `C-n` does nothing when the cursor is already at the last line, so a
reader reaching the bottom by holding `C-n` can't step past it into the place
where typing appends. A reader who expects the down arrow to keep going into
the empty space below the text (as most editors allow) finds it cannot.

## Guide-level explanation

With a file open, holding `C-n` or the down arrow stops on the last line and,
with one more press, moves onto the empty row below it. Putting the cursor there
and typing appends a new line at the end of the file. Putting it on the last
line and pressing `End` then typing also appends, but through the line's end
rather than a new row; both work, and neither is special.

`C-x C-e` (to the buffer end) goes to this row too, at column `0`: the place
where typing appends a new line. This is a deliberate change from the current
behavior, where it goes to the end of the last line; the two are one question --
what "the end of the buffer" means -- and are answered the same way.

`--tail` (`-t`) opens at this row: the empty row after the last line, which is
the place where typing appends a new line. This is a deliberate change from the
current behavior, where `--tail` opens at the end of the last line (column
`cols(last_row)`). A reader who wants the current placement can open, then press
`C-x C-e` then `End`; a reader opening with `--tail` to append with a new line
gets it in one step.

An empty file opens at row `0` and can be moved down onto row `1`, the same
empty row every file has below its text.

## Reference-level explanation

### What "the row after the last line" is

The row after the last line is `rows()`, the value that `TextBuffer::rows()`
returns today as a count. To make it a place the cursor can stand, the buffer
gains a model of it:

- **Model the buffer as `rows()` real lines plus one end row.** Concretely,
  `rows()` keeps meaning "the number of real lines" and `last_row()` keeps
  meaning `rows() - 1` (saturating, so an empty file's is `0`). The row after
  the last line is `rows()` itself, so no new accessor is needed: the cursor
  bound in `State::row_count()` becomes `rows()` instead of `last_row()`, which
  is the one change every cursor handler already draws its bound from
  (`src/state.rs`).
- The row has no `TextLine`: `line(rows())` returns `None`, `cols(rows())`
  returns `0`, and `adjust_to_char_boundary` leaves a position there at column
  `0`, exactly as they already do for a row that does not exist. So the existing
  accessors need no change; only the cursor's bound moves off `last_row()`.

This keeps the two accessors honest: `rows()` is still the count of real lines,
and `line(rows())` is still `None`. The cursor is allowed onto a row with no
line, in the same spirit as a column with no character.

**Why not a separate `end_row()`.** It would be a second name for the same
number, and the disagreement it was meant to name -- `rows()` "the count"
against "the row a cursor may reach" -- is a disagreement about meaning, not
about value. One name with a documented meaning is the smaller answer, and the
reload is the one caller that needs `last_row()` instead.

### Handlers that change

Every cursor handler that bounds a row through `State::row_count()` picks up
the new bound with no per-handler change, which is the reason `row_count()`
exists (the bug's Notes record that the same wrong bound was copied into seven
handlers). The handlers whose *meaning* of the bound changes are:

- `handle_cursor_down`: clamps at the end row so the last line's down arrow
  reaches the empty row below it.
- `handle_cursor_buffer_end` (`C-x C-e`): goes to the end row, column `0` -- the
  place typing appends a new line.
- `handle_cursor_right`: wraps to the next line while `cursor.row < rows()`, so
  right at the end of the last line moves onto the empty row.
- `handle_cursor_to_screen_position` and `handle_cursor_to_position`: clamp to
  the end row, so a click below the text and an out-of-range position land on
  it.

Decided: `C-x C-e` moves to the end row, at column 0. The tests
`an_open_at_the_file_end_lands_on_the_last_line_with_no_blank_rows`,
`holding_down_stops_on_the_last_line`, `a_click_below_the_buffer_lands_on_the_last_line`,
and `the_start_position_is_clamped_to_the_buffer` pin the old meaning and are
inverted, not deleted, so the new bound is pinned.

Handlers that read a row from outside and must not invent one are unchanged:
`handle_buffer_reload` clamps to `last_row()` (`row_count()` is the end row now,
and a reload must not leave the cursor below the text after the text changed).
The viewport caps change with it: every `(row_count() + 1) - available_rows`
becomes `row_count() - (available_rows - 1)`, which is the same number once the
bound has moved but says what the cap is -- the page that ends on the last row
a cursor may stand on -- instead of adding a row the cursor could not reach.

### Editing on the empty end row

Inserting text with the cursor on the end row appends a new line. `insert_char_at`
and `insert_newline_at` already push empty lines while `pos.row >= len()`, so no
buffer change is needed for insertion; the row becomes a real line and grows.

Deleting is the new part. Backspace on the end row is a no-op: there is no line
to join, and the existing `pos.row > 0` branch would otherwise try to join the
phantom row onto the last line. Forward delete on the end row removes the
newline that ends the last line -- its mirror -- but only when the last line is
blank: a last line with text on it is content that this press would otherwise
delete unseen. The cursor keeps its row, which is the end row again one row
higher, so it does not appear to move.

### `--tail`

`open_position` in `src/main.rs` returns `usize::MAX, usize::MAX` for `--tail`,
and the clamp in `handle_cursor_to_position` brings it to the last line and that
line's end. With the bound moved, the row clamp lands on the empty row and the
column clamp (`cols(rows()) == 0`) lands at column `0`. So `--tail` opens on the
empty row, not at the end of the last line. The flag's rustdoc, and the `--tail`
tests that currently pin the end of the last line, must say so.

Decided: the flag moves. The docs that call `--tail` "the position a named
`:MAX` already reaches" stay true -- `:MAX:MAX` moves with it, because both are
the same clamp -- so the change is one description in `open_position` and the
startup tests, not a second startup path.

### Rendering

- **Edit mode.** The terminal's own cursor marks the position, as everywhere
  outside a search, so the empty row needs no painted cell; the cursor sits on a
  blank cell below the text.
- **Search mode.** `render_line` paints a reversed blank when the cursor is past
  a line's width while a search is open. The end row has no line at all, and
  `render_text_area` walks `0..rows()` calling `line()` and skipping `None`, so
  the end row is not drawn by that loop. A search-open cursor on the end row
  needs a reversed blank painted at that row (column 0, plus the gutter when
  shown). This is the one rendering addition the proposal needs: after the
  loop, if the prompt is open and the cursor is on `rows()`, paint the gutter
  row and one reversed blank at `HIT_GUTTER_COLS`. The loop's own end row
  (a local `end_row` that meant "where the drawn slice stops") is renamed
  `slice_end` so it cannot be mistaken for the buffer's.

### Recentering and scrolling

The file-end cap on the viewport is `last_viewport = row_count() -
(available_rows - 1)`, in `adjust_viewport`, `handle_scroll`, and
`recenter_place`. It says "the last page ends on the last row a cursor may stand
on", which is now the end row, so the cap already lets the viewport show the
empty row when the cursor is on it. The three places are written the same way on
purpose -- they had to agree before, and still do. `C-l`'s "bottom" place and
the automatic recenter keep their meaning; what changes is the number they land
on near the end (the last page of a 20-line file in a 5-row area is 16, not 15).

## Drawbacks

- Re-opens a closed bug with the same tension it named: `line(rows())` is `None`
  while the cursor may be there. The design answers it by modelling the row
  explicitly rather than by pretending a line exists, but a reader who knows the
  old bug will have to read why it is now intended.
- Every handler and test that was tightened against `last_row()` has to be
  re-derived, and the tests that "the cursor cannot reach the row after the last
  line" are inverted rather than deleted, so the new bound is pinned rather than
  left implicit.
- `--tail` changes behavior for existing users: a script or habit that opened
  with `--tail` and typed into the last line now types into a new line. That is
  the point of the proposal, but it is a visible break.
- The end row is one more place editing code has to be correct about, and
  insertion/deletion there is new behavior that must be tested.

## Rationale and alternatives

- **Do nothing.** The cursor stops at the last line. Consistent with the bug's
  own reading of `rows()`, but leaves the empty-file and end-of-line
  inconsistencies and the down-arrow dead end.
- **Keep the cursor on real lines, and give `--tail` the end row only.** Smaller
  change, but then `C-n` still cannot reach the row `--tail` opens on, and the
  empty file still differs from a non-empty one. Rejected as solving the flag
  but not the request.
- **Model an explicit trailing empty line in the buffer.** Push a line when the
  file ends in a newline (or always), so `line(rows())` exists and there is no
  cursor/row mismatch at all. This is the cleanest inner model, but it changes
  `rows()` to count the trailing line, which ripples into every use of `rows()`
  as a slice end (render, totals) and into `to_text`. The proposal prefers the
  smaller change of letting the cursor reach a row with no line. If that smaller
  change proves fragile in editing, this is the fallback to reconsider.
- **Let the cursor reach the end row but keep `--tail` and `C-x C-e` on the
  last line.** Smaller, and leaves the flag and the binding where they are
  today. Rejected: both are named "the end of the buffer", so teaching them a
  different end from the one `C-n` reaches would only move the inconsistency
  from the cursor bound into two named places. Keeping them is also the only
  reading under which the change is purely additive, so it is the fallback if
  the visible break turns out to matter more than the consistency.
- **Rename rather than add.** The alternative to a new `end_row()` is to redefine
  `rows()` to mean the last addressable row and add `line_count()` for the
  count. That is a larger repaint of the buffer's API; `end_row()` adds one name
  and leaves `rows()` as documented.

## Unresolved questions

- **The name and shape of the end row.** `end_row()`, or redefine `rows()`, or a
  `TextBuffer::position_after_last_line()`? Settle before implementation.
- **Search mode on the end row.** Should a search be allowed with the cursor on
  the empty row (nothing to match, so the query simply finds nothing below the
  text)? The rendering addition above assumes the cursor can be there with a
  prompt open. (Leaning yes: the row is a position like any other, and the
  reversed blank is needed anyway for the end-of-line cursor.)

## Future possibilities

- If the inner model later grows a real trailing empty line (the alternative
  above), the cursor bound and `line()` stop disagreeing and `end_row()` becomes
  ordinary.
- `--tail` could later gain a mode that opens *and keeps* the cursor following
  the end as the file grows, which wants the same end row.
