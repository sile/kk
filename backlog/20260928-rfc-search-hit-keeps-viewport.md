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
search steps and the startup position recenter. The startup position is a
different case and keeps its centering: the named position is the reason the
file was opened, it happens once, and a file opened at a named position reads
best with that position in the middle. A search step is a move like any other,
and one step of a walk through hits should not re-frame the screen.

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

They should instead move the cursor and ask for the viewport to follow it,
rather than for it to be centered:

```rust
self.cursor = item.start_position;
self.viewport_follow = ViewportFollow::KeepVisible;
```

The name of the field and its variants is a placeholder (see the end of this
section); what matters is that the search step stops asking for a recenter.

The picture is made slightly awkward by where the size lives. `State::handle_search_next_hit`
and `handle_search_prev_hit` take no arguments, and the only place a size
reaches `State` is `adjust_viewport`, called later from the render path. `App`
does have the size and computes it lazily in `text_area_region()`
(`self.driver.size().to_region().drop_bottom(2)`). So two shapes are available:

- **Pass the size in.** The `App` arm for `Action::SearchNextHit` (and
  `SearchPrevHit`) computes `self.text_area_region().size` and calls
  `self.state.handle_search_next_hit(size)`, and the handlers take
  `text_area_size: tuinix::Size` and decide themselves. This is a signature
  change to two public methods.
- **Record the intent, not the size.** Replace the flag with a state that says
  *how* the viewport should follow, so `adjust_viewport` -- which already has the
  size -- makes the decision.

**The second shape is the one chosen.** It keeps the handlers' signatures, keeps
the size in the one place that already has it, and keeps the decision in
`adjust_viewport`, where visibility is already a question about the viewport and
the text area. The first shape would put a size argument on methods whose only
job is to move the cursor, and would duplicate the visibility rule outside
`adjust_viewport`.

The field becomes a two-variant enum rather than a `bool`:

```rust
/// How the next viewport adjustment should move the viewport.
enum ViewportFollow {
    /// Scroll just far enough to keep the cursor visible.
    KeepVisible,
    /// Center the cursor, whether or not it is already visible.
    Recenter,
}
```

`adjust_viewport` matches on it: `Recenter` does what the `true` case does
today and clears back to `KeepVisible`, and `KeepVisible` runs the existing
scroll rule whether the cursor is visible or not -- a cursor already inside
the viewport is left alone by that rule anyway, so a search hit that is already
visible needs no separate branch. The startup position and `C-l` set
`Recenter`; the search steps set `KeepVisible` explicitly rather than just
leaving the field alone, so a `Recenter` that some earlier command asked for
cannot ride along with a search step (the field is reset by every adjustment
today, but relying on that couples the search steps to when the render path
happens to run).

What must not change is where the *decision* is made: whether a position is
visible is a fact about the viewport and the text area, and both live where
`adjust_viewport` already is.

The request has two setters besides the search steps, which together account
for all six places the field is written: the startup position
(`handle_cursor_to_position`) and `C-l` (`handle_view_recenter`). Only the four
search-step assignments should lose the unconditional centering. `C-l` is an
explicit "put my line in the middle" command and must keep recentering even
when the cursor is already visible, and the startup position is deliberately
centered for the reason in the motivation. The mouse handlers do not write the
field at all, so clicks are unaffected either way.

The field is public, and two tests read it directly: `tests/search.rs` asserts
a hit recenters the view, and `tests/state.rs` asserts the request is used once.
The first assertion describes behavior this proposal changes, so it has to be
rewritten; the second survives in spirit (the request is still consumed by the
next adjustment) but may need its names updated. Either shape touches the
public API -- the first changes two method signatures, the second changes the
field's type -- so under the crate's 0.x rules both are allowed, but neither is
invisible.

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
- **More state than a `bool`.** The flag becomes a two-variant enum, which is a
  concept more than `true`/`false` for a cosmetic improvement. It also changes a
  public field's type.
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

The startup position keeps its unconditional centering: it is a one-time
"open here" rather than a step in a walk, and a file opened at a named position
reads best with that position in the middle of the text area. That is settled.

One point remains open for implementation:

- The exact name of the enum and its variants (`ViewportFollow` with
  `KeepVisible`/`Recenter` above is a placeholder). The crate's docs are the
  authority on the public API, so the name has to read well in that doc comment
  and in the two tests that mention it.

## Future possibilities

If a `C-l`-like command ever wants "always recenter" and a hit-step wants
"recenter only when needed", the flag's meaning will have to be spelled out
somewhere both can read; the third-state shape above is the natural place, and
this change is what would introduce it.
A follow-up could make the search prompt show whether the current hit is the
first or last, which composes with keeping the viewport still: a reader who is
not being recentered needs some other cue that the walk wrapped.
