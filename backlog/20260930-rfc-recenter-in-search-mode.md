# RFC: Let `C-l` recenter from the search mode

- Status: implemented

## Summary

`C-l` (`Action::ViewRecenter`) is bound in the edit mode and in no other. In a
search the reader is stepping through hits, and the moment a recenter is most
wanted -- a hit that is barely on screen, or one the minimal scroll has left
against an edge -- is exactly the moment the key is not bound: `C-l` in
`Mode::Search` resolves to nothing, and the message says so. This proposal binds
`C-l` in the search mode to the same action, so a centered view is one key away
while the prompt is open.

The row goes next to `C-s next`, the key the reader just pressed to get to the
hit.

It also grows the recenter request from a boolean into a place, so a single key
can ask for the cursor at the top or the bottom of the text area as well as in
the middle. That is what makes the search-mode binding worth having (a second
`C-l` in a row is otherwise a no-op) and it is the same cycle the sibling RFC
`20260928-rfc-recenter-cycles-through-positions.md` proposes for the edit mode;
the two should land together, and that RFC owns the type.

## Motivation

`resolve_search` binds no recenter at all:

```rust
// src/binding.rs, resolve_search
(true, tuinix::KeyCode::Char('g')) => then(Action::SearchCancel, Mode::Edit),
(false, tuinix::KeyCode::Enter) => then(Action::SearchFinish, Mode::Edit),
(true, tuinix::KeyCode::Char('y')) => act(Action::ClipboardPaste),
(true, tuinix::KeyCode::Char('k')) => act(Action::SearchCutQuery),
(true, tuinix::KeyCode::Char('s')) => act(Action::SearchNextHit),
(true, tuinix::KeyCode::Char('r')) => act(Action::SearchPrevHit),
// ... cursor moves, deletions, and the query-editing fallback
_ => return None,
```

Meanwhile the search steps deliberately do *not* center a hit that is already
visible (settled in
`20260928-rfc-search-hit-keeps-viewport.md`): a step scrolls by the minimum that
brings the hit into the text area, and only a hit more than a screen away is
centered, by the automatic rule settled in
`20260929-rfc-recenter-when-cursor-jumps-far.md`.

The minimum is the right default -- it keeps the surrounding lines still -- but
it leaves the hit at whichever edge it entered through, which for a long jump
is a poor place to read from. Today the reader's options are to keep stepping
until a later hit happens to land mid-screen, or to leave the prompt (`Ent` or
`C-g`), press `C-l`, and search again. Neither is what a reader wants in the
middle of a scan through a file for some word.

Two more things push the same way:

- The prompt is where the *cursor* is meaningful. Outside a search, `C-l`
  centers on the cursor, which is wherever the reader last put it. Inside one,
  the cursor is on a hit, so `C-l` centers on a hit -- which is what the reader
  would have had to reconstruct by hand from `C-s` presses.
- `C-l` is already a "put the interesting thing in the middle" key everywhere
  else in kk, and the search mode is missing it for no reason other than that
  nobody bound it. The mode already borrows `C-a`, `C-e`, `C-b`, `C-f`, `C-d`,
  `C-h`, `C-k`, `C-y`, and the legend lists them; `C-l` being absent is the
  anomaly.

## Guide-level explanation

With a search prompt open, `C-l` moves the viewport so the cursor -- the hit
under it -- is in the middle of the text area. Pressing it again moves the hit
to the top of the text area, and a third press puts it at the bottom, cycling
back to the middle on the fourth. The column is centered on every press, so the
cycle is a vertical one; a hit near the start or end of the buffer clamps and
the next press still advances.

The cycle is the one the edit mode has, and the two are deliberately the same:
`C-l` means the same thing in both modes, and a reader who has learned it in
one does not have to learn a second behavior in the other. The difference is
only what it centers on -- the cursor -- which in a search is the hit.

What this does *not* change is how hits are stepped. `C-s`/`C-r`/`Tab` keep the
minimum scroll, so a run of hits inside one screen still does not move the
text. `C-l` is the deliberate "put this hit in the middle" press, and asking
for it is the only thing that recenters.

In the legend, the search mode gains one row, placed directly under `C-s next`
because the reader reaches it by stepping to a hit:

```text
│C-g cancel   │C-g cancel
│Ent ⏎ finish  │Ent ⏎ finish
│C-r ⇤ prev   │C-r ⇤ prev
│C-s ⇥ next   │C-s ⇥ next
│             │C-l recenter
│C-k cut-tail │C-k cut-tail
│C-y paste    │C-y paste
│C-a bol      │C-a bol
│C-e eol      │C-e eol
│C-b ← left   │C-b ← left
│C-f → right  │C-f → right
│C-h ⌫ bs     │C-h ⌫ bs
│C-d ⌦ delete │C-d ⌦ delete
└─── Esc ──── └─── Esc ─────
   before         after
```

The new row is 13 columns (`│C-l recenter`), the same width as the mode's
current widest rows (`│C-d ⌦ delete`, `│C-k cut-tail`), so the box does not
grow -- unlike the edit legend row added for `C-j`, which was 14 and pushed the
box a column wider. The bottom border is already 13 columns
(`└─── Esc ─────`), which is the widest row in the search legend, so it stays
as it is and no border or padding changes.

## Reference-level explanation

One new arm in `resolve_search`, placed so it reads as the hit-stepping group's
neighbour -- directly after the two step bindings it belongs beside:

```rust
fn resolve_search(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { ctrl, code, .. } = *key;

    Some(match (ctrl, code) {
        // ...
        (true, tuinix::KeyCode::Char('s')) => act(Action::SearchNextHit),
        (true, tuinix::KeyCode::Char('r')) => act(Action::SearchPrevHit),
        (true, tuinix::KeyCode::Char('l')) => act(Action::ViewRecenter),
        (false, tuinix::KeyCode::Tab) => act(Action::SearchNextHit),
        // ...
    })
}
```

`Action::ViewRecenter` needs no new handling: `handle_view_recenter` sets the
pending request and the next `adjust_viewport` applies it, and a search prompt
being open changes none of that. That is the whole reason the binding is a
one-line change rather than a new action: the search mode already routes every
key through the same `State`, and the viewport arithmetic does not know a
prompt exists.

The request itself has to grow, though, or the binding is only useful once.
Today it is a flag:

```rust
pub recenter_viewport: bool,
```

and `handle_view_recenter` sets it to `true`, so a second press re-centers an
already-centered cursor and the cycle has nowhere to go. The sibling RFC
`20260928-rfc-recenter-cycles-through-positions.md` replaces the flag with
`Option<Recenter>`:

```rust
/// The place a recenter request will put the cursor on the next viewport
/// adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recenter {
    /// Cursor in the middle of the text area.
    Center,
    /// Cursor on the first visible row.
    Top,
    /// Cursor on the last visible row.
    Bottom,
}
```

and `handle_view_recenter` becomes the place that advances the cycle. This RFC
does not restate that design; it depends on it and adds the one thing the
sibling does not decide -- that the cycle is the same in the search mode.

Things that are subtle and easy to get wrong:

- **Where the cycle state lives.** `handle_view_recenter` runs before any
  `adjust_viewport`, so it has to decide the next place from what is
  observable. Two of the three places are readable from the viewport and the
  cursor (`viewport.row == cursor.row` is `Top`, and
  `viewport.row == cursor.row - (text_rows - 1)` is `Bottom`), but the text
  area's rows are only known at render time. The sibling RFC's preference -- an
  explicit "place last asked for" field -- is the one to take, because it also
  removes the ambiguity when two places coincide (a text area one row tall
  makes `Center` and `Top` the same viewport; a field still advances, a
  derivation cannot tell them apart). It landed as
  `last_recenter: Option<Recenter>`, where `None` opens the cycle at `Center`
  so the first press does what it always did.
- **Coincident places in a search.** A search hit at row 0 of a file, in a text
  area shorter than the file, makes `Center` and `Top` the same viewport: the
  first press appears to do nothing, the second lands on `Bottom`. With a
  "last place asked for" field this is only a cosmetic oddity; the shared field
  is what makes it possible to decide at all, and the sibling RFC's unresolved
  question about skipping coincident places applies here unchanged.
- **The gutter changes `text_rows`.** While a search prompt is open the summary
  rows take rows from the text area, and which summary rows are shown depends
  on the viewport. `adjust_viewport` already settles the two together with a
  fixed-point loop, so a `Top` request that puts a summary row above the
  viewport is drawn in the reduced area on the next pass; a recenter branch
  that returns early must keep using `text_rows()`/`text_cols()` rather than the
  raw `text_area_size`, exactly as the current `Center` branch does.
- **Horizontal centering.** The proposed cycle centers the column on every
  place, so `Top` and `Bottom` differ from `Center` in rows only. A hit near
  the right edge of a long line is centered horizontally by the same press,
  which is wanted -- the hit is the whole point of the view.
- **Empty text area.** The arithmetic stays `saturating_sub`-based, so
  `rows == 0` cannot underflow (the current `Center` branch already relies on
  this).
- **The message.** `handle_view_recenter` reports `"View recentered"`. With
  three places the message names the place (`"Cursor centered"`,
  `"Cursor at top"`, `"Cursor at bottom"`), shared with the edit mode rather
  than decided here.

## Tests

- `tests/binding.rs`: `C-l` resolves to `Action::ViewRecenter` in both the edit
  and search modes, and each mode's legend names the chord it binds.
- `tests/legend.rs`: the search legend's exact rendering gains the `C-l` row
  under `C-s`; the existing width and border checks pin that the box stayed 13
  columns wide.
- `tests/state.rs`: the cycle visits center, top, and bottom and wraps; a `Top`
  request puts the cursor on the first drawn row and a `Bottom` request on the
  last; a startup position rewinds the cycle so a later `C-l` centers again.

## Drawbacks

- **A binding in the mode whose keys are scarcest.** The search mode's keys are
  all taken by query editing and hit stepping, and `C-l` is one more contested
  chord. Unlike the edit mode there is no `C-x` prefix to move it under, so the
  choice is this chord or none.
- **The legend row moves the search legend's widest-row count from two to
  three.** Nothing grows -- the row is 13 like the others -- but the box's width
  is now pinned by three rows instead of two, so a future label change has less
  slack before the box widens.
- **The cycle's third press moves the hit to the bottom of the screen**, which
  for a reader stepping hits is a less useful place than the middle; the cycle
  is inherited from the edit mode rather than designed for search, and a
  two-place cycle would arguably suit search better. Landing on the same
  behavior in both modes was judged worth more than the extra state fitting
  search perfectly.
- **A key that used to do nothing does something now.** A reader who hit `C-l`
  inside a search and read the unbound message will find the key now moves the
  text; the message was at least honest, and the change is what was asked for.

## Rationale and alternatives

- **Do nothing.** A reader who wants a centered hit can leave the prompt, press
  `C-l`, and re-enter. Rejected: the reader is often mid-scan, and the surface
  area of the prompt is exactly where the recenter should live; the edit mode
  already has the key and the mode's cursor is already on the thing worth
  centering.

- **Add only the binding, keep `bool`.** The smallest change that satisfies the
  literal request. Rejected on its own: a second `C-l` is a no-op, and the
  sibling RFC is already replacing the field, so the binding can have the cycle
  for the same amount of work. If the sibling RFC is rejected, this collapses to
  this option, and the binding is still worth having.

- **A search-only place** (for example, a `C-l` that always centers, even if
  the edit mode keeps the cycle). Rejected: the same key meaning two different
  things in two modes the reader can switch between with one chord (`C-s`,
  `Ent`) is a worse surprise than an imperfect cycle.

- **A separate key for the search-mode recenter** (for example, `Tab`-based or
  a new mode-prefixed chord). Rejected: `Tab`, `BackTab`, and the arrows are
  already bound to hit stepping, and a new chord would have to be learned; the
  key that already means "recenter" is the one to bind.

- **Recentering on every step instead** (making `C-s` center, as kk did before
  `20260928-rfc-search-hit-keeps-viewport.md`). Rejected there and still wrong:
  it shuffles the text under the reader on every press, which is what the
  minimum scroll was adopted to stop. `C-l` being explicit is what keeps both
  behaviors available.

- **Recentering when the hit is barely visible** (a threshold on the step, so a
  hit that lands against an edge is centered). This is the closest alternative,
  and it is tempting because it needs no key. Rejected because the threshold is
  a guess: a hit one row inside the edge looks the same as a hit one row
  outside, which the automatic far-jump rule already centers, so a second
  threshold would decide the same case twice and could disagree with the first.
  Making the reader ask is unambiguous and leaves the step's rule alone.

## Unresolved questions

- Whether the cycle skips a place that coincides with the previous one. The
  same question is open in the sibling RFC; the answer should be shared, not
  decided twice, and the search mode (which can center a hit at row 0 or on the
  last row) is where a user is most likely to see the oddity. The landed code
  advances regardless, matching the edit mode.
- Whether `Top`/`Bottom` should center the column as well, or align the line to
  the left edge. Centering landed because the hit is what is being read;
  left-aligning would show more of a long line's tail past the hit.

## Future possibilities

Once the binding exists, a search-mode key that jumps straight to a place rather
than cycling (one key for `Top`, another for `Bottom`) becomes possible without
touching the cycle, because the places are already values in `Recenter` rather
than a flag. If a later command wants "put the hit at the top" as a *step*
behavior rather than a manual press, that is a fourth place in the enum and a
second pending request -- not a change to this binding.
