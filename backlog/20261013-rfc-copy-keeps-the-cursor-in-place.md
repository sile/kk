# RFC: A copy keeps the cursor where it is

- Status: draft

## Summary

A copy (`C-x w`) currently leaves the cursor at the start of the region it
copied, exactly as a cut (`C-w`) leaves it at the start of the region it cut.
That symmetry is wrong for a copy: a cut deletes the region and the cursor sits
where the text used to begin, but a copy changes nothing, so moving the cursor is
a jump the user did not ask for. This proposal keeps the cursor where it was
when the copy ran, and keeps the mark-clearing question separate.

## Motivation

Mark a block, copy it with `C-x w`, and the cursor jumps to the top of the block.
The text is exactly as it was -- that is what a copy is -- so nothing the user is
looking at has moved, and yet the cursor has. On a long region, the copy drags
the cursor (and, through it, the viewport) back to a place the user had already
scrolled past, and the next keystroke acts somewhere else than it would have
without the copy.

The cut's cursor move is defensible: cutting *removes* the region, and the
natural place to leave the cursor is where the removed text began, so the buffer
reads continuously across the cut. A copy removes nothing, so there is no
corresponding "where the text began" to settle on -- the cursor was already
somewhere sensible, and moving it is a second effect of a command whose whole
point is to have *no* effect on the buffer.

The current behavior is documented, so this is a deliberate choice being
revisited rather than an accident: the copy RFC says the cursor moves "exactly as
after a cut", and the handler sets `self.cursor = start` to match.

## Guide-level explanation

Mark a region and copy it with `C-x w`; the cursor does not move. Marking, then
copying, then pasting with `C-y` puts the text back at the cursor, which is now
the same place the copy started from -- not the region's start.

The mark is still cleared, exactly as today. Nothing else about `C-x w` changes
either: it still fills kk's in-process clipboard, still writes the region to the
terminal's clipboard as OSC 52, still reports `Copied N characters`, and still
reports `No mark set` / `Nothing to copy` for an empty region. A cut (`C-k`, `C-w`) is unchanged -- it still moves the cursor to
where the cut text began.

```text
before:   [foo]
            ^ cursor
mark the whole line with C-g, then copy with C-x w

today:    foo
          ^ cursor jumped to the start

proposed: foo
             ^ cursor stays where it was
```

## Reference-level explanation

In `State::handle_mark_copy` (`src/state.rs`), the copy currently does two things
after it has the region: it writes the text to the clipboard, then moves the
cursor and clears the mark:

```rust
self.clipboard.write(&text);
self.cursor = start;
self.mark = None;
```

The proposal drops the cursor assignment:

```rust
self.clipboard.write(&text);
self.mark = None;
```

`start` (`take_mark_region`'s first return value) is bound for nothing once the
cursor stops using it, so the pattern's first binding is renamed `_start` and sits
next to the already-unused `_end`. The character count the message reports comes
from `text`, not from `start`, and `take_mark_region` still owns the empty-region
check, so neither needs the position back. The rest of the handler is unaffected.

The mark is still cleared, exactly as it is today: only `self.cursor = start` is
removed, and `self.mark = None` stays. The cursor move and the mark-clear happen
to sit next to each other in the handler, so a reader changing one must not
disturb the other; the intent is narrow -- *the cursor* moves off the copy, the
mark behavior does not.

The message stays `Copied N characters`. It is computed from the region's
character count, which the cursor change does not touch, so nothing about it
moves.

The `start` value the cut uses (`handle_mark_cut`) is unchanged; only the copy
handler stops writing to `self.cursor`.

## Drawbacks

- The cut and the copy stop leaving the cursor in the same place, so the two
  handlers differ by one line (a side effect) as well as by the delete. That is
  the intended difference, but it is one more place to explain why a copy and a
  cut are not the same shape.
- The copy RFC (`backlog/done/20261002-rfc-a-copy-leaves-to-the-terminal-clipboard.md`)
  describes the old "moves to the region's start" behavior, so its text is
  superseded here for the cursor. That is a record of what was decided then,
  not something to edit; the doc comment on `handle_mark_copy` is updated as
  part of the change.

## Rationale and alternatives

- **Leave it as it is.** The symmetry with a cut is easy to state and the jump
  is one keystroke to undo. Rejected because this is a copy: its value is that
  nothing about the buffer (including where the user is) moves, so a cursor jump
  works against the command's purpose rather than with it.
- **Move the cursor to the region's *end* instead of its start.** Keeps some
  symmetry with a cut while matching the "leave the text as it was" reading.
  Rejected: it still moves the cursor, which is the complaint, and the end is no
  less arbitrary than the start once the text is not removed.
- **Keep the mark after a copy.** A copy leaves the buffer untouched, so a case
  can be made for leaving the mark too, letting a second `C-x w` re-copy without
  re-marking. Rejected for this change: it is a wider behavior change than the
  cursor move being reported, and a copy that clears the mark matches what a cut
  does today, so only the cursor differs. Revisit separately if re-copying turns
  out to be wanted.
- **Do nothing.** The cursor keeps jumping on every copy. Rejected: the jump is
  the thing being reported, and it is a small, local change to remove.

## Impact

Ergonomics, and small. One assignment (`self.cursor = start`) is removed from one
handler, its doc comment is corrected, and the copy RFC's description is
superseded for the cursor. The clipboard, the OSC 52 export, the messages, the
bindings, the legend, and the cut are all unchanged.

## Dependencies

None. No new dependency and no change outside `src/state.rs` (plus tests).

## Testing

- The core test that pins the copy's cursor position should assert the cursor is
  *unchanged* by `handle_mark_copy`, where it currently asserts it moved to the
  region's start. See `tests/state.rs` around the `handle_mark_copy` cases.
- The existing "a copy differs from a cut only in the delete" framing in tests is
  exactly what changes, so any test asserting the copy's cursor equals the cut's
  cursor is the one to revisit.
- No end-to-end change: the OSC 52 export and the message are untouched.

## Unresolved questions

None. The mark is cleared (unchanged from today) and the message stays
`Copied N characters`; both are settled in "Reference-level explanation".
