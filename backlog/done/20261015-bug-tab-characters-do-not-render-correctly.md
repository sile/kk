# Bug: Tab characters do not render correctly

- Status: fixed

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

- The fix was made in the width rule the column model reads, not in
  `render_line`: `render_line` still positions every character from the
  column the buffer reports, so the two agree by sharing one definition
  rather than by teaching the renderer a second one. `render_line` was left
  alone.
- The tab width was a literal `8` inside `put_str` and nowhere else; it is
  now named once and both the renderer and the column model read it, so a
  later change to the width cannot split them again.
- The search highlight and the gutter count columns through the same
  `char_cols()` and `contains()`/`count_up_to()`, and they inherited the same
  shift. The search prompt measures columns itself as well, so it had to be
  moved onto the new rule directly, not only the buffer.
- Whether the tab should be *drawn* (as `^I`, or a `→` up to the stop) rather
  than as blanks is a presentation choice. It is not needed to make the columns
  agree; if it is wanted, it is a follow-up rather than part of this bug.

## Outcome

Fixed in [#22](https://github.com/sile/kk/pull/22) (merged as `6ccee36`).

`src/terminal.rs` now names the tab width once, in `TAB_WIDTH`, and
`put_str` reads it instead of a literal `8`. A new `char_cols_at(ch, col)`
returns a tab's width as the distance from `col` to the next tab stop and any
other character's width from `char_cols`, so a tab is the columns up to the
next multiple of the width wherever it starts. Both are public, since the
buffer needs the same rule the renderer draws with.

Every place the buffer asked `char_cols` how wide a character is now asks
`char_cols_at` with the column the character starts at -- all of `TextLine`'s
column-aware methods (`char_cols()`, `width()`, `char_at_col()`,
`adjust_to_char_boundary()`, `delete_char_at()`, `find_char_before()`,
`insert_char_at()`, `char_index_at_col()`, `col_at_char_index()`) and the
cursor advance in `TextBuffer::insert_char_at_internal`. Column arithmetic
that is already per-character is unaffected; only the width lookups moved.
The search prompt measured columns for its highlight with the same call and
moved with them, so the highlight and the gutter's totals follow a tab to the
same columns the text is drawn at.

`render_line` was not touched: it places each character at the column the
buffer reports and `put_str` writes nothing for a tab, so a tab now leaves
the blank columns it takes and the characters after it land where the cursor
and the marks put them.

`tests/render.rs` gained three cases: a line that begins with a tab draws its
text eight columns in, a tab mid-line moves the rest of the line to the next
stop, and the cursor on the tab's first column paints there -- the cell the
tab claims, which the buffer and the renderer now both agree is the tab's.

The scope is unchanged from what is described above.
