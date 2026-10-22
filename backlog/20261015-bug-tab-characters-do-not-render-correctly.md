# Bug: Tab characters do not render correctly

- Status: open

## Summary

A tab in the buffer is given zero display columns, but the renderer advances
past it by a tab stop. A line that holds a tab therefore draws its text past
the tab in the wrong columns -- while the buffer's own column model counted the
tab as nothing -- and the drawn line does not say what the buffer holds.

The smallest case is a file with one tab-indented line.

## Reproduction

```text
$ printf '\tindented\n' > /tmp/tab.txt
$ kk /tmp/tab.txt
```

Observed: the leading tab contributes no column of its own, so `indented` is
stuck to the left edge where the tab should have pushed it eight columns right.
The tab is also not drawn as anything, so the line reads as `indented` with no
indication that a tab came before it.

The same holds for a tab mid-line (`a\tb`): `b` is drawn one column past `a`,
because the buffer said the tab was zero columns wide, while the renderer's
tab stop believes it moved to column eight. The two disagree by eight columns.

## Observed behavior

Two pieces of the code disagree about how wide a tab is.

`src/buffer.rs` measures every character with `crate::terminal::char_cols`,
which is `unicode_width`'s width. That returns `0` for a tab, so every
column-aware method in `TextLine` -- `char_cols()`, `width()`,
`char_index_at_col()`, `col_at_char_index()`, `adjust_to_char_boundary()`, the
insert/delete helpers -- treats a tab as occupying no column:

```rust
pub fn char_cols(&self) -> impl Iterator<Item = (usize, char)> {
    let mut col = 0;
    self.0.iter().map(move |&ch| {
        let current_col = col;
        col += crate::terminal::char_cols(ch);
        (current_col, ch)
    })
}
```

`src/render.rs`'s `render_line` walks those pairs and paints each character at
`text_offset + (current_col - start_col)`: the column the buffer chose. It calls
`put_str` one character at a time, and `put_str` handles a tab the opposite
way -- it moves the pen to the next tab stop and writes nothing:

```rust
'\t' => {
    if let Some(tab) = std::num::NonZeroUsize::new(8) {
        at = at.next_tab_stop(tab);
    }
}
```

So the position `put_str` lands on is thrown away (`render_line` computes each
character's position from the buffer, not by chaining `put_str`'s return), and
the cell the tab should have occupied is left unwritten. The columns after it
are painted at the buffer's zero-width-for-tab positions, so the rest of the
line is shifted left by the tab's width, and the tab itself shows as nothing.

`src/terminal.rs`'s own rustdoc on `put_str` promises "tabs advance to the next
tab stop", and `tuinix::Char::new` refuses control characters, so a frame can
never hold a tab as a glyph. The only consistent reading is that a tab is a
run of blank columns up to the next tab stop, which is what `put_str` says and
what `TextLine`'s column model does not.

## Expected behavior

A tab occupies the columns from its start up to the next multiple of the tab
width, and those columns are drawn as blanks. After that, the characters
that follow a tab land at the columns the same rule gives them, so the
buffer's column model and the drawn line agree -- a cursor moved onto a tab or
past it sits in the cell it is drawn in, and a copy or cut of a range that
crosses a tab carries the tab through unchanged.

Tab width is a display decision that already exists in `src/terminal.rs`
(`put_str` uses 8); the buffer's column model must use the same value, from one
definition rather than two copies.

## Impact

Correctness for anyone who opens a file that contains tabs -- which for source
code is most files in a directory. The cursor and the drawn line disagree about
where a column is, so moving over a tab lands the cursor in the wrong cell, and
the text after a tab is silently shifted. Reading such a file in `kk` is the
whole point of the tool, and the tab width it shows is not what it reports.

The bug is not about *inserting* a tab: `src/binding.rs`'s `resolve_edit`
accepts no control character as text (`(false, KeyCode::Char(ch)) if
!ch.is_control()`), so the key event for a tab is `BufferSave` and no tab can
be typed in. Only the tabs already in the file reach the buffer.

## Notes

- The fix belongs in `TextLine`'s width rule, not in `render_line`: if the
  buffer counted a tab as eight columns, `render_line` would place the
  following characters at those columns and `put_str` would have nothing to
  advance over -- a tab would be painted as one glyph of some width, which is
  not what characters past a tab stop need. The decision is *where the columns
  are*, and that lives with `TextLine`.
- The tab width is currently a literal `8` inside `put_str` and nowhere else;
  the fix should name it once and have both the renderer and the column model
  read it, so a later change to the width cannot split them again.
- Related, but separate: the search highlight and the gutter count columns
  through the same `char_cols()` and `contains()`/`count_up_to()`, so they
  inherit the same shift. Fixing the width rule fixes them together, since
  they compare `TextPosition`s built from the one column model.
- Whether the tab should be *drawn* (as `^I`, or a `→` up to the stop) rather
  than as blanks is a presentation choice. It is not needed to make the columns
  agree; if it is wanted, it is a follow-up rather than part of this bug.
