# RFC: Make `C-l` cycle the cursor through center, top, and bottom

- Status: draft

## Summary

`C-l` (`Action::ViewRecenter`) currently centers the cursor and does nothing
observable when pressed again, because centering is idempotent. Make repeated
`C-l` presses cycle the cursor through a fixed sequence of places -- centered,
then the top of the text area, then the bottom, then centered again -- the way
Emacs `recenter` does. The request is to reuse the recenter machinery kk
already has (`State::recenter_viewport`), not to add a second mechanism beside
it.

## Motivation

`State::recenter_viewport` is a boolean that asks the next `adjust_viewport` to
center the cursor and clear itself. (A sibling RFC,
`20260928-rfc-search-hit-keeps-viewport.md`, wants the search steps to stop
setting it, so this RFC turns the flag into `Option<Recenter>` and the search
steps write `None` -- see "The flag is shared" below.)

```rust
// src/state.rs
if self.recenter_viewport {
    self.viewport.row = cursor_pos.row.saturating_sub(available_rows / 2);
    self.viewport.col = cursor_pos.col.saturating_sub(available_cols / 2);
    self.recenter_viewport = false;
    return;
}
```

`handle_view_recenter` sets it and reports `"View recentered"`. Pressing `C-l`
once does what it says; pressing it again re-centers an already-centered
cursor, so the second and every later press are no-ops. There is no way to ask
for the cursor at the top or the bottom of the text area, even though the
minimal-scroll rule below the flag will happily leave the cursor against an
edge as a side effect of an unrelated move.

The 3-state cycle is not new in this family of tools. `mamediff`'s
`Action::Recenter`, bound to the same kind of key, already cycles
`center -> top -> bottom`:

```rust
// ../mamediff/src/app.rs (verbatim)
fn recenter(&mut self) {
    if self.terminal.size().is_empty() {
        return;
    }

    let current = self.frame_row_start;
    let cursor_row = self.tree.cursor_row();
    let top = cursor_row;
    let bottom = cursor_row.saturating_sub(self.terminal.size().rows - 1);
    let center = cursor_row.saturating_sub(self.terminal.size().rows / 2);
    self.frame_row_start = if current != center && current != top {
        center
    } else if current == center {
        top
    } else {
        bottom
    };
}
```

kk and `mamediff` share the author and the "keep the cursor visible" model, so
matching this behavior keeps a `C-l` between the two tools predictable.

## Guide-level explanation

`C-l` cycles the cursor through three places, in this order, wrapping around:

1. **Center** -- cursor in the vertical middle of the text area (today's only
   behavior).
2. **Top** -- cursor on the first visible row.
3. **Bottom** -- cursor on the last visible row.

Pressing `C-l` at the top of the cycle goes back to center, so any place is at
most three presses away and the key never becomes a dead end the way it is
today.

A short file, or a position close to either end of the buffer, clamps: asking
for center near the top of a 10-line file leaves the viewport at row 0 and the
cursor stays where it is, and the cycle still advances on the next press
because the state tracks the *request*, not the resulting viewport. Cases that
cannot be distinguished (top and center coincide) are described under
Unresolved questions.

## Reference-level explanation

`recenter_viewport: bool` says *that* a recenter is pending, not *which* place
was asked for, and the cycle needs the place. Replace the flag with an enum
that carries both:

```rust
/// The place a recenter request will put the cursor on the next viewport
/// adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recenter {
    /// Cursor in the middle of the text area (current `C-l` behavior).
    Center,
    /// Cursor on the first visible row.
    Top,
    /// Cursor on the last visible row.
    Bottom,
}
```

The field becomes `recenter_viewport: Option<Recenter>`, where `None` means no
recenter request is pending -- today's `false`, which runs the ordinary
keep-it-visible rule (scroll by the minimum needed to show the cursor). The
search steps land on exactly that behavior by setting `None`, so `None` carries
both "nothing asked" and "follow with the minimum scroll". `adjust_viewport`
matches on it instead of an `if`:

```rust
if let Some(place) = self.recenter_viewport.take() {
    let (rows, cols) = (available_rows, available_cols);
    self.viewport = match place {
        Recenter::Center => TextPosition {
            row: cursor_pos.row.saturating_sub(rows / 2),
            col: cursor_pos.col.saturating_sub(cols / 2),
        },
        Recenter::Top => TextPosition {
            row: cursor_pos.row,
            col: cursor_pos.col.saturating_sub(cols / 2),
        },
        Recenter::Bottom => TextPosition {
            row: cursor_pos.row.saturating_sub(rows.saturating_sub(1)),
            col: cursor_pos.col.saturating_sub(cols / 2),
        },
    };
    return;
}
```

`handle_view_recenter` advances the cycle. It has to read the *current* place to
know the next one, and the place that was last applied is gone from the field
(`adjust_viewport` took it), so the cycle is decided from what is already
observable -- the viewport against the cursor -- the way `mamediff` does:

```rust
pub fn handle_view_recenter(&mut self) {
    self.finish_editing();
    let rows = /* last known text area rows */;
    self.recenter_viewport = Some(self.next_recenter(rows));
    self.set_message(match self.recenter_viewport {
        Some(Recenter::Top) => "Cursor at top",
        Some(Recenter::Bottom) => "Cursor at bottom",
        _ => "Cursor centered",
    });
}
```

Things that are subtle and easy to get wrong:

- **Where the cycle state lives.** `handle_view_recenter` runs before any
  `adjust_viewport`, so it must predict the next place from the viewport and
  cursor as they are now. Deriving "am I centered / at top / at bottom" from
  `viewport.row - cursor.row` against the text area's rows reproduces
  `mamediff`'s `current != center && current != top` test, but the text area's
  size is only known at render time. Either store the last text area size in
  `State` (set by `App::render` before `adjust_viewport`), or keep an explicit
  `recenter_index`/`last_place` field that records the place last asked for,
  which is simpler to reason about than re-deriving it. The explicit field is
  preferred: it removes the same ambiguity `mamediff` has when top and center
  coincide.
- **The flag is shared.** The startup position (`handle_cursor_to_position`)
  and the search steps (`handle_search_next_hit`, `handle_search_prev_hit`)
  also write `recenter_viewport`. The startup position wants a one-shot
  *center*, so it sets `Some(Recenter::Center)` and is unaffected by the cycle.
  The search steps want the viewport to follow the hit with the minimum scroll,
  not to center, so they set `None` -- the same value that means "no request"
  today, and the sibling RFC's whole point. Neither caller participates in the
  cycle.
- **Horizontal centering.** Today `recenter` centers both axes. The proposal
  keeps the column centered for all three places, so only the vertical
  position changes across the cycle; a top/bottom request is about rows. If a
  later request wants per-axis places, the enum grows two components instead of
  one.
- **Empty text area.** `adjust_viewport` is still called with `rows == 0` at
  startup in some paths; the arithmetic must stay `saturating_sub`-based so a
  zero-size area cannot underflow.

## Drawbacks

- More state than a boolean: a public enum, a field that is now `Option<Recenter>`
  rather than `bool`, and the cycle bookkeeping. `C-l` is currently the simplest
  action in the table, and this makes it the most stateful.
- The third press changes where the cursor sits relative to the file's start
  and end, which interacts with the "scroll just far enough" rule: after a
  cycle, the next cursor move re-adjusts from a non-minimal viewport, so the
  screen can jump more than a single-line move would suggest.
- A reader who learned `C-l` as "recenter" now has to learn that it cycles;
  the third state (bottom) is a new concept kk did not have.

## Rationale and alternatives

- **Do nothing.** `C-l` is already useful once; the second press being a no-op
  is harmless. Rejected because the key is cheap to press and the cycle is what
  makes it worth pressing more than once, which is the whole request.

- **A separate action for top and bottom** (e.g. `M-r`/`M-b`, or a numbered
  prefix). Would avoid overloading `C-l`, but adds bindings and a legend row for
  something Emacs-style editors fold into one key; kk has no prefix arguments,
  so there is nowhere natural to put the choice.

- **Keep `bool` and derive the cycle from the viewport** (exactly `mamediff`'s
  approach). Fewer fields, but it needs the text area's rows inside `State` and
  it is ambiguous when top and center coincide; the explicit `Option<Recenter>`
  is only a little more code and removes both problems.

- **Only cycle between center and top**, leaving bottom out. Simpler, but
  bottom is the state that the minimal-scroll rule already produces implicitly,
  so exposing it makes the rule explainable rather than a surprise.

## Unresolved questions

- Whether to keep an explicit "which place was requested last" field or to
  derive it from the viewport and cursor. The RFC leans explicit; the derived
  form matches `mamediff` and needs no extra field.
- What to do when two places coincide. On a text area one row tall, or when the
  cursor is at row 0, top and center are the same viewport; cycling through both
  still advances the state, but the user sees one press do nothing. Decide
  whether the cycle skips coincident places or advances regardless.
- Whether the horizontal column should stay centered on the top/bottom places
  (proposed) or align to column 0 / the line's end.
- Whether the message should name the place (`"Cursor at top"`) or stay the
  single `"View recentered"` string.
- The search steps writing `None` is settled by the sibling RFC; what is not is
  whether anything *else* should follow the hit instead of centering. If a
  later command wants "put the hit at the top", that is a fourth place and the
  enum grows rather than the search steps changing again.

## Future possibilities

If `Recenter` becomes public, the CLI-position jump could expose the place as a
flag (`--recenter=top`), and a `M-l`-style repeat key could jump straight to the
next place without cycling from center. A per-axis place enum would also let a
future horizontal recenter ("cursor at the window's middle column") reuse the
same field.
