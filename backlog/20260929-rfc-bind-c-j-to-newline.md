# RFC: Bind `C-j` to newline in the edit mode

- Status: implemented

## Summary

`Enter` inserts a newline in the edit mode, but `C-j` does nothing: it resolves
to no action at all, so a reader who reaches for it out of Emacs habit gets a
silent no-op. This proposal binds `C-j` to the same `Action::NewlineInsert`
that `Enter` already runs, and gives the newline binding a row in the edit
legend.

## Motivation

In the edit mode, `resolve_edit` has no arm for a ctrl chord on `j`:

```rust
// src/binding.rs, resolve_edit
(true, tuinix::KeyCode::Char('f')) => act(Action::CursorRight),
(false, tuinix::KeyCode::Delete) => act(Action::CharDeleteForward),
(false, tuinix::KeyCode::Backspace) => act(Action::CharDeleteBackward),
(false, tuinix::KeyCode::Enter) => act(Action::NewlineInsert),
```

The catch-all for text requires `ctrl == false`:

```rust
(false, tuinix::KeyCode::Char(ch)) if !ch.is_control() => act(Action::CharInsert),
```

so `C-j` falls through to `None` and the edge reports it as an unbound input.
The newline is the one edit a reader reaches for most often, and `C-j` is the
chord Emacs binds it to -- `RET` and `C-j` are both `newline` there, kept as a
pair because a terminal cannot always tell the two apart and because a reader
may have either in their fingers. kk already honours the `RET` half; this
proposal adds the `C-j` half so the pair travels together as it does in the
editor the chord comes from.

Because `C-j` is unbound today, no existing binding has to move and nothing a
reader already types changes meaning. The chord is free.

## Guide-level explanation

In the edit mode, `C-j` and `Enter` both split the line at the cursor:

```text
before:                      after C-j (cursor between `al` and `pha`):

  al|pha beta                  al
                               |pha beta
```

Nothing else changes. In the search prompt `C-j` stays unbound, as it is today;
the prompt's `Enter` is `SearchFinish`, and the prompt has no newline of its own
to insert. The extension mode is untouched.

The edit legend gains one row so the new binding is discoverable the same way
the others are. It reads `C-j ⏎ newline`:

```text
│C-l recenter
│C-j ⏎ newline
│Tab ⇥ save
```

The ⏎ (RETURN SYMBOL) matches the way ⌫ and ⌦ stand in for backspace and
delete, and ⇥ for tab: a glyph beside the chord, then the label. The row is
placed with the other cursor-and-edit commands, next to `C-l`, rather than at
the very bottom, so the legend still ends with `Tab` (save) over its border.

## Reference-level explanation

One arm is added to `resolve_edit` in `src/binding.rs`, beside the `Enter`
arm it duplicates:

```rust
(true, tuinix::KeyCode::Char('j')) => act(Action::NewlineInsert),
```

No new `Action`, no new handler, and no core change: `handle_newline_insert`
already does exactly what is wanted, and the edge already dispatches
`Action::NewlineInsert` to it. The binding is the whole change to behaviour.

The edit legend in `src/binding.rs` gains the row `│C-j ⏎ newline`. The
table is hard-coded and kept in step with the bindings by hand (its doc says
so), so the two move together in one change.

The existing tests that pin the tables have to move with it:
`the_edit_legend_is_exactly_this_text` in `tests/legend.rs` spells every row
out, so it gains the new row and, if the row is inserted mid-list, the
padding of the rows below shifts to the new widest width. The legend width is
the widest row, and `Tab ⇥ save` is 11 columns before the left border;
`C-j ⏎ newline` is 14, one wider than the previous widest row (`C-k cut-tail`
and `C-d ⌦ delete`, both 13), so the box grows by one column and every other
row's right padding grows with it. A test asserting that `C-j` resolves to
`Action::NewlineInsert` in the edit mode should be added beside
`undo_is_bound_to_ctrl_u_alone`.

## Drawbacks

- **A second chord for one action.** `Enter` and `C-j` now do the same thing,
  which is one more binding to keep in step by hand and one more row in a
  legend the crate already keeps manually. The related search-legend bug
  `20260929-bug-search-legend-omits-enter.md` settles the spelling for the
  chord there: the chord column spells the key short, `Ent`, and the label is a
  word for what it does, `finish`.
- **`C-j` is a control character byte (`0x0a`), the same byte as a bare
  newline.** A terminal that sends the raw line feed without a ctrl modifier
  would not match the `ctrl == true` arm; it would fall to the `Enter` code
  only if the decoder labels it so. This is the same ambiguity Emacs lives
  with, and it is why the pair is kept together rather than either alone, but
  it means `C-j` is only as reliable as the terminal's decoding of `0x0a`.
- **The current behavior is defensible.** "One key, one newline" is a simple
  rule, and `Enter` is on every keyboard.

## Rationale and alternatives

- **Do nothing.** `Enter` already inserts a newline and is universally
  available. Rejected because `C-j` is the Emacs newline chord and kk borrows
  the rest of its edit chords from Emacs; leaving `C-j` an unbound no-op
  punishes exactly the reader the other bindings were chosen for.
- **Bind `C-j` in every mode.** Rejected: the search prompt's `Enter`
  finishes the search, and a newline is not something the prompt edits. The
  extension mode has no text to edit. Only the edit mode has a buffer to split.
- **Swap the two, making `C-j` newline and dropping `Enter`.** Rejected: `Enter`
  is the key every keyboard has and the one a new reader will try first; the
  proposal adds a chord rather than moving one.
- **Leave the legend as it is.** The legend's doc says the table is kept in
  step with the bindings, so a binding with no row breaks that promise. Adding
  the row is what keeps the table honest.

## Unresolved questions

None. The ⏎ glyph is kept as written: it matches the way ⌫, ⌦, and ⇥ stand in
for keys the reader does not type literally. The spelling convention is settled
by `20260929-bug-search-legend-omits-enter.md`: the chord column shortens the
key the way the other rows shorten theirs, and the label is a word for what the
key does rather than a repeat of the chord. Whether `Enter` itself should also
get a row in the edit legend (it is bound but unlisted today, the same way it
was unlisted in the search legend) is a separate question and is not decided
here.

## Future possibilities

If a later binding wants to insert a newline without splitting the line -- a
soft wrap, say -- the two arms would stop sharing one action and the legend
row would have to name which one `C-j` is. Nothing today needs that.
