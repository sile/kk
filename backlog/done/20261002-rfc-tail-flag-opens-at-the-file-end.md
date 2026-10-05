# RFC: A --tail flag opens at the file end

- Status: accepted

## Summary

Opening a file puts the cursor at the start, and `FILE:LINE:COLUMN` moves it
somewhere named on the command line. Neither helps the common case of opening a
log or any other file whose interesting end is at the bottom: the reader opens
the file and immediately presses `C-x C-e` to get to the end. A `--tail` flag
(`-t`) opens the file with the cursor already at the end, so the same file can
be opened at the end by habit rather than by a second keystroke.

## Background

The command line is parsed in `src/main.rs`. `--create-new` is the pattern a
new flag follows: it is taken before the positional, so a trailing `-c` is not
read as the `FILE`. The positional is split by `split_position` into a path and
an optional `:LINE[:COLUMN]`, and the position is handed to `App::new`, which
calls `State::handle_cursor_to_position` to put the cursor there. That call
centers the cursor in the first frame (see the RFC on opening at a CLI
position), so a named line lands in the middle of the text area.

`handle_cursor_to_position` clamps an out-of-range position against the buffer
rather than rejecting it: the row is clamped to the buffer's last line and the
column to that line's width. A position deliberately past both bounds -- the
clamp's own limits -- is therefore the file's end: the last line
(`last_row() = rows() - 1`, what `row_count()` returns) and that line's last
column. So `FILE:MAX:MAX` already opens a file at its end today, by way of the
clamp every position goes through.

The end of the file is also reached after opening by `C-x C-e`
(`handle_cursor_buffer_end`), but that lands on the *start* of the last line
(column `0`), not its end; reaching the file end to append is `C-x C-e` then
`End` (`handle_cursor_line_end`).

## Proposal

`-t` / `--tail` takes no argument and means: after the file is loaded, move the
cursor to the end of the last line. It is defined as the position `FILE:MAX:MAX`
already names -- there is no new cursor position and no second path. Both the
row and the column are the file's end -- the last line, `rows() - 1`, and that
line's last column, `cols(rows() - 1)` -- so a reader who opens with `--tail`
and starts typing appends at the end of the file, where they would have been
after pressing `C-x C-e` and then `End`.

When both `--tail` and a `FILE:LINE:COLUMN` position are given, the position is
ignored and the file opens at the end. The two name the same thing -- where to
put the cursor on open -- and `--tail` is the more explicit request, so it wins
rather than making the pair an error. A named position is an ordinary way to
open a file, so rejecting it whenever `--tail` is also present would turn a
working invocation into a failure for no gain.

`--tail` is placed exactly as any other startup position, by the same
`handle_cursor_to_position` call and the same centering: it is the startup
position with the end's value, nothing more. The centering does not spend the
lower half of the frame on blank rows even at the file end: the recenter that
handles the startup position caps the place at the file's last page, so a
cursor at the end lands on the last drawn row with the lines above it filling
the rest of the frame (the file-end rule settled in the companion RFC on
recentering near the file end). Opening at the end and pressing `C-l` to settle
the placement reach the same spot, because both go through the same capped
center.

## Design

`--tail` is a boolean, like `--create-new`, taken in `src/main.rs` before the
positional:

```rust
let tail = noargs::flag("tail")
    .short('t')
    .doc("Open at the end of the file")
    .take(&mut args)
    .is_present();
```

The flag does not reach `App::new`: it is applied in `src/main.rs`, before the
position is 1-based-to-0-based and handed over. When `--tail` is set, the
`LINE` and `COLUMN` from the command line are overwritten with `usize::MAX`
there, so `App::new` still takes only the position and a `--tail` open reaches
it as the position `FILE:MAX:MAX` would carry. The rest of the path is
unchanged -- the position still goes to `State::handle_cursor_to_position`,
which clamps it against the buffer:
`row.min(...)` and `col.min(...)` bring a `usize::MAX` row to the last line and
a `usize::MAX` column to the end of that line. So the file end is reached by
the clamp the core already applies to every out-of-range position, not by a
new call, and the last-line row and the end-of-line column fall out of one
value with no branch to pick between them.

There is no second signal for the placement: `--tail` is the ordinary startup
position, so it takes the ordinary startup recenter (the capped center). The
flag does not reach the recenter at all -- only the position does, and it is
the position `FILE:MAX:MAX` would carry.

This is why the named position is simply overwritten rather than dropped after
the fact. Overwriting is what makes the two flags compose to "go to the end"
with no case for the pair: `--tail` sets the position, and a `LINE`/`COLUMN` on
the command line is set aside by the same overwrite, not by a separate check.
The path is still taken from the positional -- `split_position` keeps splitting
the `:LINE[:COLUMN]` and the path half is used whether `--tail` is given or not,
so the path parsing stays one rule for every invocation.

### Empty and short files

An empty file is one empty line, so its end is row `0`, column `0`, and
`--tail` opens there, the same place as the start. No special case is needed:
the `usize::MAX` clamp already brings the cursor to whatever the last line is,
and the last line of an empty file is row `0`. A file with fewer lines than the
frame is the ordinary near-the-end case, and the capped center places the
cursor on the last drawn row as it would for any other startup position.

One detail to settle in the implementation: clamping `col` to the last line's
width and then snapping to a character boundary, in the order
`handle_cursor_to_position` already uses, should leave the cursor at the line's
end. The RFC relies on that order; if it does not hold, the column needs its own
step rather than falling out of the clamp.

### Relation to `--create-new`

`--tail --create-new` opens an empty file at the end, which is its start. This
is consistent: the file has no other line to open at, and the two flags do not
conflict. No check is needed between them.

## Alternatives

- **Call `handle_cursor_buffer_end` instead of overwriting the position.** The
  end of the file has its own handler (`C-x C-e`), and calling it would name the
  destination directly rather than routing through the clamp. It is also the
  wrong place: `C-x C-e` lands on the start of the last line (column `0`), not
  its end, so it would not put the cursor where typing appends. Overwriting the
  position is smaller and lands both the row and the column from one value,
  the end of the last line where `C-x C-e` then `End` would leave the cursor.
- **Reuse `FILE:LINE:COLUMN` and tell the reader to compute the last line.**
  The reader does not know the line count before opening, and `C-x C-e` already
exists for after opening. The flag is what makes it one step at open time.
- **Make `--tail` follow the file as it grows** (like `tail -f`). That is a
different feature with a different cost (watching the file, redrawing on
change) and is not what "open at the end" needs. If it is wanted later it can be
its own proposal.
- **Give `--tail` its own placement, on the last drawn row.** This was the
  first shape of the design, on the belief that centering the end would spend
  the lower half of the frame on blank rows. It would not: the startup centering
  is already capped at the file's last page, so it lands on the last drawn row
  with no blank rows below it, the same as the automatic recenter's file-end
  rule. Nothing needs the separate placement, so `--tail` takes the ordinary
  startup centering.
- **Error when `--tail` and a position are both given.** The pair is not
  contradictory -- both name where the cursor goes -- and a named position is a
  normal way to open a file, so failing would reject a reasonable command line.
  Ignoring the position is enough.

## Impact

Ergonomics. No new cursor position and no new editing behavior: the file opens
at the position `FILE:MAX:MAX` already names, placed by the startup recenter
that already handles every other opening position. The cost is one more flag on
the command line and one more argument through `App::new`.

## Outcome

Implemented in [#16](https://github.com/sile/kk/pull/16) (merged as `7ae2806`).

`--tail` / `-t` was added as a boolean flag in `src/main.rs`, taken before the
positional like `--create-new`, and `open_position` (a new pure helper) maps
the parsed position to the 0-based `tuinix::Position` the open needs: under
`--tail` it is `usize::MAX, usize::MAX`, otherwise the parsed row and column
shifted to 0-based. The flag does not reach `App::new` or the recenter; it is
folded into the position, so the file end is reached by the clamp every
out-of-range position already goes through, exactly as the RFC describes.

The position is not overwritten before the shift but after it, so `--tail` is
`usize::MAX` and not one short. The RFC's Design was tidied in the same pull
request to match: `App::new` still takes only the position, and the "second
signal for a non-centred placement" the text had sketched was dropped, since
the capped startup centre already leaves no blank rows at the end.

Tests: `split_position` and `open_position` unit tests in `src/main.rs` cover
the flag and the shift, and a `tests/state.rs` test pins that
`handle_cursor_to_position(usize::MAX, usize::MAX)` -- what `--tail` produces --
lands at the end of the last line with no blank rows below. `cargo test`,
`cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings` pass.

The scope is unchanged from what is described above.
