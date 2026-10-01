# RFC: Make `C-l` cycle the cursor through center, top, and bottom

- Status: accepted

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
center the cursor and clear itself. A sibling RFC,
`20260928-rfc-search-hit-keeps-viewport.md`, already had the search steps stop
setting it, so the flag is now written only by `C-l` and by the startup
position.

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

The request stays a boolean. Which place it lands on is worked out from the
viewport, the way `mamediff` does it, so nothing remembers a place:

```rust
/// Where a recenter request puts the cursor's row in the text area.
///
/// The three are the places `C-l` cycles through. Which one a press asks for
/// is read off the viewport it is pressed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecenterPlace {
    Center,
    Top,
    Bottom,
}

impl RecenterPlace {
    /// The viewport row that puts the cursor's row at this place.
    fn row(self, cursor_row: usize, available_rows: usize) -> usize {
        match self {
            Self::Center => cursor_row.saturating_sub(available_rows / 2),
            Self::Top => cursor_row,
            Self::Bottom => cursor_row.saturating_sub(available_rows.saturating_sub(1)),
        }
    }

    fn message(self) -> &'static str { /* "Cursor centered", ... */ }
}
```

The enum is private. Nothing outside `State` needs to name a place: the field
is public but holds a boolean, and the place is an implementation detail of
the key.

`handle_view_recenter` only raises the request. An earlier draft had it also
pick the place and write the message, which forced the text area's height into
`State` (or an estimate of it) so the press could tell "already centered" from
"already at the top". It does not know the height, and guessing gets the cycle
wrong: with the estimate one row short of the real area, the third press asked
for the bottom and the adjustment sized it as a center. Deciding the place in
`adjust_viewport`, which has the height, removes the third state entirely.

```rust
pub fn handle_view_recenter(&mut self) {
    self.finish_editing();
    self.recenter_viewport = true;
}
```

`adjust_viewport` takes the request, picks the place from the viewport as the
last adjustment left it, and sizes that place inside its existing fixed-point
loop (rather than an `if` that returns early), so it lands against the drawn
height the summary rows leave:

```rust
let recenter = self.recenter_viewport;
self.recenter_viewport = false;
let recenter_place = if recenter {
    let available_rows = self.text_rows(text_area_size.rows);
    Some(self.recenter_place(available_rows))
} else {
    None
};

for _ in 0..3 {
    let available_rows = self.text_rows(text_area_size.rows);
    let before = self.viewport.row;

    if let Some(place) = recenter_place {
        self.viewport.row = place.row(cursor_pos.row, available_rows);
    } else {
        // ... the keep-it-visible rule, unchanged
    }

    if self.viewport.row == before {
        break;
    }
}

if recenter {
    if let Some(place) = recenter_place {
        self.set_message(place.message());
    }
}

// The column is centered when a place asked for it, and scrolled by the
// minimum otherwise.
if recenter_place.is_some() {
    self.viewport.col = cursor_pos.col.saturating_sub(available_cols / 2);
} else if /* ... the keep-it-visible column rule */ {
    // ...
}
```

`recenter_place` is the whole cycle:

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

Things that are subtle and easy to get wrong:

- **The top is tested before the center.** The places are not disjoint: a
  cursor in the middle of the drawn rows is on the top row and centered at the
  same time whenever the area's height has not changed since the last
  adjustment. Testing the center first leaves the cycle there, and the third
  press never reaches the bottom.
- **The place is chosen once, not per loop pass.** The loop overwrites
  `viewport.row` on its first pass, so a second pass branching on the new value
  steps the cycle a second time: the first press would center and then move on
  to the top in the same adjustment. The place is picked before the loop; only
  the row it works out to is re-evaluated against each pass's height.
- **The startup position is not a `C-l` press.** `handle_cursor_to_position`
  wants a one-shot *center* and must not step the cycle, or the reader's first
  `C-l` after opening a file at a `FILE:ROW:COL` position would jump to the top
  instead of centering. It writes its own `center_viewport` flag, which asks
  for `RecenterPlace::Center` directly. Keeping it apart also keeps the
  message off the startup path: it is not a place the reader asked for, so it
  should not be announced, and the message is written only for a `C-l` press.
  The search steps (`handle_search_next_hit`, `handle_search_prev_hit`) want
  the viewport to follow the hit with the minimum scroll, so they leave both
  flags alone.
- **Horizontal centering.** Today `recenter` centers both axes. The proposal
  keeps the column centered when a place asked for it, so only the vertical
  position changes across the cycle; a top/bottom request is about rows. If a
  later request wants per-axis places, `RecenterPlace` grows two components
  instead of one.
- **Empty text area.** `adjust_viewport` is still called with `rows == 0` at
  startup in some paths; the arithmetic must stay `saturating_sub`-based so a
  zero-size area cannot underflow. Folding the place into the existing
  fixed-point loop, rather than returning after sizing it once, is what keeps a
  `Bottom` request honest when the summary rows change the drawn height.

## Drawbacks

- A second boolean on `State` (`center_viewport`, for the startup position),
  which exists only so the startup path does not step the cycle. The cycle
  itself needs no memory, but the two kinds of request do have to be told
  apart.
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

- **Carry the place in the request** (`recenter_viewport: Option<Recenter>`,
  with an explicit `last_recenter` to advance the cycle). This is what the
  first draft did. It keeps the cycle out of `adjust_viewport`, but it needs
  the text area's height inside `State` to work out the next place -- or an
  estimate, which is what the draft used and which is wrong: the cycle reads
  the same viewport the adjustment is about to rewrite, so the two disagree,
  and the press asks for one place while the adjustment applies another.
  Deriving the place from the viewport in `adjust_viewport` removes both the
  extra state and the disagreement.

- **Only cycle between center and top**, leaving bottom out. Simpler, but
  bottom is the state that the minimal-scroll rule already produces implicitly,
  so exposing it makes the rule explainable rather than a surprise.

## Unresolved questions

None. Each press advances one place, including a place that coincides with the
previous one on screen, and a place the buffer cannot show -- a `Bottom` with
fewer than a text area of rows above the cursor -- clamps at the buffer's first
row rather than scrolling past it; the cursor then sits somewhere other than the
last drawn row, which is the nearest the request can be met.

## Future possibilities

If `Recenter` becomes public, the CLI-position jump could expose the place as a
flag (`--recenter=top`), and a `M-l`-style repeat key could jump straight to the
next place without cycling from center. A per-axis place enum would also let a
future horizontal recenter ("cursor at the window's middle column") reuse the
same field.

## Outcome

Implemented in [#11](https://github.com/sile/kk/pull/11) (merged as `ba66c26`).

## Outcome

Implemented in PR #11, merged as `ba66c26`.

`C-l` now cycles the cursor through center, top, and bottom, naming the place it
moved to on the message line so the cycle is legible without counting presses.

Rejected, after building it: threading an `Option<Recenter>` request through
state with a remembered `Option<Recenter>` last-press field. That design has the
press handler pick the next place from an estimate of the geometry (it has no
`text_rows`), while `adjust_viewport` applies it from the real one. The two drift
apart, and the third press could name "bottom" and land on center. The shipped
design instead derives the next place from the viewport at the moment the
request is applied -- first drawn row means top, last drawn row means bottom,
otherwise center -- so there is no second source of truth. Top is tested before
center because the two coincide when the text area is as tall as the buffer;
center first would stick there. The request goes back to a plain `bool`, so the
startup position and a far cursor jump, which also mean "center", are unchanged.

`Top` and `Bottom` clamp at the buffer edge: with a terminal taller than the
cursor's distance from either end, the viewport stops at the edge and the cursor
does not reach the text area's last row. That is all a view can do, and the
rustdoc says so.

Unanticipated: the request did not need to grow from `bool` into
`Option<Recenter>` in the state after all, and `Recenter` did not need to become
public -- it is a private `RecenterPlace`. The existing `ViewRecenter` doc and
the legend rustdoc were stale and needed touching.

Coverage: `tests/state.rs` exercises the cycle (center, top, bottom, and back),
`Top` landing on the first drawn row, `Bottom` on the last drawn row, a shared
cycle across the two modes, and a zero-height guard.

The scope is unchanged from what is described above.
