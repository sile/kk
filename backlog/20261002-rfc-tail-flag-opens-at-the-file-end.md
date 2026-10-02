# RFC: A --tail flag opens at the file end

- Status: open

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

The end of the file is reached by `handle_cursor_buffer_end` today, the same
place `C-x C-e` goes. It currently lands on the row after the last line, which
is the subject of the bug on the cursor reaching the row past the last line;
this proposal assumes that bug is fixed and the cursor goes to the last real
line.

## Proposal

`-t` / `--tail` takes no argument and means: after the file is loaded, move the
cursor to the end of the last line, as `C-x C-e` does once the bug above is
fixed. Both the row and the column are the file's end -- the last line,
`rows() - 1`, and that line's last column, `cols(rows() - 1)` -- so a reader who
opens with `--tail` and starts typing appends at the end of the file, where they
would have been after pressing `C-x C-e` and then `End`.

When both `--tail` and a `FILE:LINE:COLUMN` position are given, the position is
ignored and the file opens at the end. The two name the same thing -- where to
put the cursor on open -- and `--tail` is the more explicit request, so it wins
rather than making the pair an error. A named position is an ordinary way to
open a file, so rejecting it whenever `--tail` is also present would turn a
working invocation into a failure for no gain.

The cursor is placed at the last drawn row, not centered, because that is what
the automatic recenter does for a jump near the file end (see the companion RFC
on recentering near the file end). The startup position is centered because a
named line has text on both sides worth showing; the file end has nothing below
it, so centering it would spend the lower half of the frame on blank rows. The
reader opening at the end asked for the end, and the end belongs against the
bottom edge, with the lines above it filling the rest of the frame.

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

The flag is passed to `App::new` beside `create_new`. Inside, it is turned into
the position to ask for: when `--tail` is set, the `LINE` and `COLUMN` from the
command line are overwritten with `usize::MAX` before they become a
`tuinix::Position`. The rest of the path is unchanged -- the position still goes
to `State::handle_cursor_to_position`, which clamps it against the buffer:
`row.min(...)` and `col.min(...)` bring a `usize::MAX` row to the last line and
a `usize::MAX` column to the end of that line. So the file end is reached by
the clamp the core already applies to every out-of-range position, not by a
new call, and the last-line row and the end-of-line column fall out of one
value with no branch to pick between them.

This is why the named position is simply overwritten rather than dropped after
the fact. Overwriting is what makes the two flags compose to "go to the end"
with no case for the pair: `--tail` sets the position, and a `LINE`/`COLUMN` on
the command line is set aside by the same overwrite, not by a separate check.
The path is still taken from the positional -- `split_position` keeps splitting
the `:LINE[:COLUMN]` and the path half is used whether `--tail` is given or not,
so the path parsing stays one rule for every invocation.

The one thing the overwrite does not decide is the recenter. The startup path
centers (`center_viewport`), because a named line has text on both sides worth
showing; `--tail` wants the file-end rule from the companion RFC, which places
a jump near the file end on the last drawn row. So the flag needs a second
signal beside the position -- telling the recenter that this is a file-end open
-- and the placement is the file-end rule, not the center. The position is the
startup one with a different value; only the placement differs.

### Empty and short files

An empty file is one empty line, so its end is row `0`, column `0`, and
`--tail` opens there, the same place as the start. No special case is needed:
the `usize::MAX` clamp already brings the cursor to whatever the last line is,
and the last line of an empty file is row `0`. A file with fewer lines than the
frame is the ordinary near-the-end case, and the end rule places the cursor on
the last drawn row as it would for any other jump.

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
  destination directly rather than routing through the clamp. Overwriting the
  position is smaller: it reuses the call the startup already makes, needs no
  branch to choose between the end and a named line, and lands both the row and
  the column from one value. The two reach the same place once the bug on the
  row past the last line is fixed.
- **Reuse `FILE:LINE:COLUMN` and tell the reader to compute the last line.**
  The reader does not know the line count before opening, and `C-x C-e` already
exists for after opening. The flag is what makes it one step at open time.
- **Make `--tail` follow the file as it grows** (like `tail -f`). That is a
different feature with a different cost (watching the file, redrawing on
change) and is not what "open at the end" needs. If it is wanted later it can be
its own proposal.
- **Center the end like the startup position.** The end has no text below it, so
  centering spends the lower half of the frame on blank rows; the file-end rule
  for the automatic recenter already places such a jump on the last drawn row,
  and `--tail` should land the same way.
- **Error when `--tail` and a position are both given.** The pair is not
  contradictory -- both name where the cursor goes -- and a named position is a
  normal way to open a file, so failing would reject a reasonable command line.
  Ignoring the position is enough.

## Impact

Ergonomics. No new cursor position and no new editing behavior: the file opens
at the position `C-x C-e` already reaches, placed by the file-end rule the
automatic recenter already uses. The cost is one more flag on the command line
and one more path through `App::new`.
