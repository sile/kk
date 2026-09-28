# RFC: Recenter the view when the CLI position puts the cursor off screen

- Status: draft

## Summary

`kk FILE:LINE:COLUMN` moves the cursor to that line and column, but the
viewport is only adjusted by the generic "scroll just far enough to keep the
cursor visible" rule on the first render. When the requested line is the first
or last line of a taller screen, that rule is happy to leave the cursor against
the top or bottom edge. The request is to center the requested position
instead, so opening a file at a known line shows that line in the vertical
middle of the text area.

## Motivation

The command line already promises more than "start editing":

```text
$ kk src/render.rs:120:1
```

moves the cursor to line 120 and column 1. What it does not do is make that
place easy to read. In `src/app.rs`, `App::new` calls
`State::handle_cursor_to_position`, which sets `cursor` and calls
`finish_editing()` -- it does not touch `viewport`. The viewport is fixed by
`State::adjust_viewport`, which runs once per render (`App::render`) and does
the minimum scrolling needed to keep the cursor on screen:

```rust
if cursor_pos.row < self.viewport.row {
    self.viewport.row = cursor_pos.row;
} else if cursor_pos.row >= self.viewport.row + available_rows {
    self.viewport.row = cursor_pos.row.saturating_sub(available_rows.saturating_sub(1));
}
```

Starting from a zeroed viewport, the first render therefore puts a requested
line either at the top of the text area (when the line is above it) or exactly
at the bottom edge (when the line is below it). The line is visible, but the
text after it -- often the reason for jumping to it -- is not, and the line sits
in a corner rather than where a reader expects a "go here" to land.

The machinery for centering already exists: `State::recenter_viewport` is a
flag that makes the next `adjust_viewport` center the cursor and clear itself.
It is what `C-l` (`Action::ViewRecenter`), a search hit, and both hit commands
set. Only the startup path does not use it.

## Guide-level explanation

Opening a file at a position should behave like opening it and then asking for
that position to be centered:

```text
$ kk big.log:5000:1
```

Before: line 5000 is painted on the last visible row, with everything after it
off the bottom of the screen.

After: line 5000 is painted in the middle of the text area, with roughly half a
screen of context above and below it. A short file, or a line already near the
middle, is unaffected in practice: `saturating_sub` clamps the viewport to the
buffer's start, so opening a 10-line file at line 5 still shows the file from
its first line.

A `FILE` with no suffix is not affected at all. There is no position to
center, and centering line 1 would only push the buffer's first line down the
screen for no reason.

## Reference-level explanation

`State::handle_cursor_to_position` asks the next viewport adjustment to center
the cursor, exactly as a search hit does:

```rust
pub fn handle_cursor_to_position(&mut self, row: usize, col: usize) {
    self.cursor.row = row.min(self.buffer.rows());
    self.cursor.col = self.buffer.cols(self.cursor.row).min(col);
    self.cursor = self.buffer.adjust_to_char_boundary(self.cursor, true);
    self.recenter_viewport = true;
    self.finish_editing();
}
```

The flag is consumed by the first `adjust_viewport` in `App::render`, before
anything is painted, so the user only ever sees the centered frame. Nothing
else changes: `adjust_viewport` already handles the flag, and the clamping it
does keeps the viewport inside the buffer.

Three things are subtle and worth stating:

- **Only when a position was given.** `handle_cursor_to_position` is called
  from `App::new` for every startup, with a zeroed position when the argument
  had no suffix. Setting the flag then would center row 0, which
  `saturating_sub` clamps to row 0 anyway, so the flag is harmless -- but the
  intent is only meaningful when the user asked for a place. Whether to guard
  on a "was a position given" signal, or to let the clamp absorb the no-suffix
  case, is left open below.

- **It is the cursor, not the line, that is centered.** `adjust_viewport`
  centers `cursor_position()` in both dimensions, so a requested column is
  centered horizontally too. `handle_cursor_to_position` snaps the column onto
  a character boundary before this, so a `:LINE:COLUMN` that lands inside a
  wide character centers on the character's start.

- **Out-of-range positions still clamp.** `handle_cursor_to_position` clamps
  the requested row to `buffer.rows()` and the column to the line's width
  before the flag is read, so `:9999` on a short file centers the clamped
  position (the row after the last line) rather than erroring, which is what
  the 1-based `FILE:LINE[:COLUMN]` parsing and its tests already promise.

## Drawbacks

- The first frame scrolls further than it strictly needs to, painting a few
  lines that the minimal rule would have kept off screen. That is the point,
  but it is a behavior change for a startup path users may have gotten used to.
- Centering the column vertically as well as horizontally may surprise anyone
  who thinks of `:LINE:COLUMN` as a vertical-only jump; the cursor is centered
  in both axes because that is what `recenter_viewport` already means.

## Rationale and alternatives

- **Do nothing (current behavior).** Defensible: the requested line is visible
  and the viewport is consistent with every other cursor move. Rejected because
  the whole reason to name a line on the command line is to look at it, and
  against a corner of the screen it is the least readable it can be.

- **Center only vertically.** Would match the intuition that `:LINE:COLUMN`
  scrolls vertically, but requires a second flag or a parameter on
  `adjust_viewport`, and `recenter` already means both axes everywhere else
  (`C-l`, search hits). Since a `:COLUMN` is also a request to look at a
  place, centering both keeps one meaning for the flag.

- **Set the viewport directly in `App::new`.** The edge would compute
  `row - rows / 2`, but the text area's size is only known at render time (the
  terminal can be resized before the first paint), and clamping is the core's
  job; reusing `recenter_viewport` keeps all viewport arithmetic in `State`.

- **A new `Action`/`State` method for startup positioning.** `handle_cursor_to_position`
  is only reached from `App::new` and its own tests, so widening it is simpler
  than adding a variant that means the same thing.

## Unresolved questions

- Whether "a position was given" should be an explicit signal (for example,
  passing `Option<tuinix::Position>` through `App::new`) or left implicit in
  the clamp. The clamp makes the no-suffix case a no-op either way, so this is
  about intent rather than behavior.
- Whether the horizontal centering is wanted for a `:COLUMN` as well as the
  vertical centering for a `:LINE`. If not, the flag needs a component for
  each axis.
- Whether the centering belongs on every startup or only when the requested
  line is the first or last visible one. Centering a line that is already in
  the middle changes nothing, so the simpler "always" is proposed.

## Future possibilities

If `FILE:LINE[:COLUMN]` grows other jump targets (a byte offset, a search
pattern), they can set the same flag and inherit the same presentation. A
future "open at the top" or "open at the center" command-line switch would be
a natural place to expose the choice this RFC settles by default.
