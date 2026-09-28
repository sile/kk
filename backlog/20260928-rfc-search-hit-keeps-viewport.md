# RFC: Keep the viewport when a search hit is already visible

- Status: draft

## Summary

Stepping through search hits with `C-s`/`C-r` (or `Tab`) recenters the viewport
on every press, even when the next hit is already on screen. This proposal keeps
the recenter only when the hit would otherwise land outside the text area, so a
run of hits inside one screen no longer scrolls the text out from under the
reader.

## Motivation

`handle_search_next_hit` and `handle_search_prev_hit` set
`recenter_viewport` unconditionally, and `adjust_viewport` centers the cursor
whenever that flag is set:

```rust
// src/state.rs, handle_search_next_hit
self.cursor = next_item.start_position;
self.recenter_viewport = true;
```

Centering is the right call when the hit is far away: the reader asked to be
taken to it, and the middle of the text area is the most useful place to land.
It is the wrong call for hits that are already visible. With a handful of hits
in one screen -- a word repeated in a paragraph, say -- each `C-s` yanks the
text a few rows up or down so that the hit sits in the middle, even though the
hit was plainly there a moment before. The reader loses the surrounding lines
that made the hit worth looking at, and has to re-read the screen after every
press.

The same is true in the other direction: `C-r` recenters too, so stepping back
through the hits the reader just passed shuffles the text again each time.

Recenter-on-every-hit is also inconsistent with how the rest of the editor
moves the cursor. `C-p`, `C-n`, `C-b`, `C-f` scroll the viewport just far enough
to keep the cursor visible and leave it alone when it already is; only the
search steps and the startup position recenter. The startup position is
arguably a different case -- the named position is the reason the file was
opened, and it belongs in the middle once -- but a search step is a move like
any other, and one step of a walk through hits should not re-frame the screen.

## Guide-level explanation

Stepping through search hits keeps the viewport still as long as the hit is
visible:

```text
before:                          after C-s (hit already visible):

  1  alpha beta gamma             1  alpha beta gamma
  2  gamma delta           ->     2  gamma delta
  3  epsilon zeta                 3  epsilon zeta
  4  eta theta                    4  eta theta
     ^ viewport                      ^ viewport unchanged
                                     ^ cursor on line 3 hit

before:                          after C-s (hit below the text area):

  1  alpha beta gamma             3  epsilon zeta
  2  gamma delta           ->     4  eta theta
  3  epsilon zeta                 5  iota kappa
  4  eta theta                   ...  centered on the hit
     ^ viewport
```

When the next hit is already inside the text area, the text does not move at
all and only the cursor jumps to it. When it is outside -- below the last
visible row, above the first, or past the right edge -- the hit is brought into
view. The wrapped cases (past the last hit back to the first, and before the
first back to the last) are ordinary moves and follow the same rule: recenter
only if the hit is not already visible.

Nothing else about the search prompt changes. `C-g` still cancels and restores
the cursor and viewport from before the prompt was opened; the hit set, the
match order, and the wrapping are untouched.

## Reference-level explanation

The change is confined to the two search-step handlers in `src/state.rs`. Today
both end by setting the flag:

```rust
// handle_search_next_hit and handle_search_prev_hit, both branches
self.cursor = item.start_position;
self.recenter_viewport = true;
```

They should instead set the cursor and then let a shared helper decide whether
the viewport has to move, and how:

```rust
self.cursor = item.start_position;
self.reveal_cursor(text_area_size);
```

The helper needs the text area size, which the handlers do not have today:
`State::handle_search_next_hit` and `handle_search_prev_hit` are called from
`App` without a size, because only `adjust_viewport` (called later, from the
render path) is given one. Two shapes are available:

- **Pass the size in.** `App` already knows the frame size where it dispatches
  the action, so the handlers can take `text_area_size: tuinix::Size` and do the
  decision themselves. This is a signature change to two public methods.
- **Record the intent, not the size.** Keep the flag but give it a third state,
  so `adjust_viewport` -- which already has the size -- makes the decision:
  instead of a `bool` meaning "center no matter what", the state carries
  "recenter if the cursor is not already visible".

Either shape works and the second keeps the handlers' signatures; the choice is
left to implementation. What must not change is where the decision is made:
whether a position is visible is a fact about the viewport and the text area,
and both live where `adjust_viewport` already is.

`recenter_viewport` has two setters besides the search steps, which together
account for all six places the flag is set: the startup position
(`handle_cursor_to_position`) and `C-l` (`handle_view_recenter`). Only the four
search-step assignments should lose the unconditional centering. `C-l` is an
explicit "put my line in the middle" command and must keep recentering even
when the cursor is already visible, and the startup position is deliberately
centered for the reason in the motivation. The mouse handlers do not set the
flag at all, so clicks are unaffected either way.

`adjust_viewport`'s keep-it-visible rule already handles both axes: it scrolls
up when the row is above the viewport, down when it is at or past
`viewport.row + rows`, and the same for columns. The new state only decides
whether that rule runs or the centering branch does, so no new scrolling logic
is needed.

## Drawbacks

- **A visible hit may land against an edge.** If the reader steps to a hit one
  row above the bottom of the text area, it stays there rather than centering,
  which some readers will find cramped. This is the cost of not moving the text,
  and it is the same trade the cursor moves already make.
- **The viewport stops being predictable from the hit.** With centering, the hit
  was always a known distance from the edges; now it depends on where it was
  already. A reader who counted on centering to orient themselves loses that.
- **More state than a `bool`.** Either shape adds a concept -- a size parameter
  threaded to two handlers, or a flag with three meanings -- for a cosmetic
  improvement.
- **The current behavior is defensible.** "The hit is always in the middle" is a
  simple rule to state and to rely on, and recentering is never *wrong*, only
  sometimes unwanted.

## Rationale and alternatives

- **Do nothing.** The current behavior is simple and self-consistent. Rejected
  because it makes a run of nearby hits harder to read than the same run walked
  with `C-p`/`C-n`, and because the fix is small and local.
- **Keep recentering, but only when the hit is off screen entirely.** This is
  very close to the proposal; the difference is that a hit just past the last
  visible row would scroll the minimum amount under the proposal, versus
  centering once it is out of view. Both are reasonable; the proposal's rule is
  the one that matches the rest of the editor's movement, since
  `adjust_viewport` already scrolls by the minimum when it has to.
- **Scroll so the hit sits at the top of the text area instead of the middle.**
  A common editor behavior, but a larger change: it affects every hit, visible
  or not, and is a different aesthetic from the rest of kk's movement. Rejected
  as not what was asked for.
- **Always keep the viewport still and never scroll on a hit.** Rejected: a hit
  off screen would then be invisible, and the reader would have to scroll to it
  by hand, which defeats the search step.
- **A count or a mode for centering ("recenter on every hit")** -- a preference
  the reader toggles. Rejected as too much machinery for a behavior kk has no
  other preferences for.

## Unresolved questions

- Whether the startup position should keep centering unconditionally. This
  proposal keeps it, on the grounds that it is a one-time "open here" rather
  than a step in a walk, but the two are closer than they look if a future
  change makes the startup position reachable from a key.

## Future possibilities

If a `C-l`-like command ever wants "always recenter" and a hit-step wants
"recenter only when needed", the flag's meaning will have to be spelled out
somewhere both can read; the third-state shape above is the natural place, and
this change is what would introduce it.
A follow-up could make the search prompt show whether the current hit is the
first or last, which composes with keeping the viewport still: a reader who is
not being recentered needs some other cue that the walk wrapped.
