# RFC: Alt-modified keys match nothing

- Status: draft

## Summary

Keys with the Alt modifier are currently ignored: every resolver drops the
`alt` field and matches on `ctrl` and `code` alone. The effect is that an
unbound chord like `M-a` is read as the bare `a` and inserts a character. This
proposal makes an Alt-modified key match no binding at all, so holding Alt
never falls back to the un-modified chord.

## Motivation

`src/binding.rs` destructures each key as
`let tuinix::KeyInput { ctrl, code, .. } = *key;`, discarding `alt`. Since
`tuinix::KeyInput` carries `alt` as a first-class field, the modifier is
present when a key arrives but is then ignored.

The result is that `M-a` is indistinguishable from `a`. In [`Edit`](Mode::Edit)
that reaches the catch-all `(false, KeyCode::Char(ch)) if !ch.is_control() =>
Action::CharInsert` and inserts `a`; in [`Search`](Mode::Search) it appends `a`
to the query. A user who presses Alt by accident, or expects an Alt chord to be
inert because it is unbound, instead edits the buffer. An unbound chord should
do nothing, not resolve to a different binding.

This is the one place where the modifier is handled inconsistently: `ctrl` is
part of the spelling and of every match arm, while `alt` is silently treated
as if it were not pressed.

## Guide-level explanation

Before:

```rust
let key = tuinix::KeyInput { ctrl: false, alt: true, code: tuinix::KeyCode::Char('a') };
let input = tuinix::Input::Key(key);
// Mode::Edit.resolve(&input)
// -> Some(Resolved { action: Some(Action::CharInsert), mode: None })
//    the bare `a` is inserted
```

After:

```rust
let key = tuinix::KeyInput { ctrl: false, alt: true, code: tuinix::KeyCode::Char('a') };
let input = tuinix::Input::Key(key);
// Mode::Edit.resolve(&input)
// -> None
//    the Alt chord is unbound, so nothing happens
```

A user thinks of an Alt chord as the chord `M-a`, which is not in the legend
and not bound. Pressing it should be a no-op, the same way `C-q` (also unbound)
is a no-op.

## Reference-level explanation

Every resolver returns `None` for a key whose `alt` is true, before the `match`:

```rust
fn resolve_edit(key: &tuinix::KeyInput) -> Option<Resolved> {
    let tuinix::KeyInput { alt, ctrl, code } = *key;
    if alt {
        return None;
    }
    ...
}
```

The same guard is added to `resolve_search` and `resolve_ext`. Nothing else
changes: `ctrl` and `code` still select the arm, and a key with `alt == false`
resolves exactly as it does today.

`fmt::display_input` already renders an Alt chord as the chord without Alt
(`src/fmt.rs`: "Alt is a modifier `kk` does not act on, so it is not part of the
spelling"). With this change the rendering and the resolution agree: an Alt
chord both looks and resolves like nothing.

## Drawbacks

- The guard is three lines in each of the three resolvers, which is a small
  amount of repetition. It could instead be a shared helper at the top of
  `Mode::resolve`, before the mode dispatch, which may be preferable.
- The change is behavior-visible only for key combinations no user is expected
  to press on purpose, so it is cheap to adopt and cheap to reverse.

## Rationale and alternatives

- **Ignore Alt (current behavior).** Rejected: it makes an unbound chord fall
  through to a different binding, which is surprising and edits the buffer.
- **Treat Alt as a first-class modifier in the spelling.** This would let `M-`
  chords be bound later and matches how `C-` is handled. It is more work than
  this proposal needs, because no Alt binding is planned; the guard can be
  replaced by real `M-` arms when one is. Doing nothing but the guard keeps the
  door open without committing to a spelling or a legend entry yet.
- **Do nothing.** The current behavior is documented in `src/fmt.rs`, so it is
  defensible as a rendering rule, but the resolver behavior it implies (an Alt
  chord acts as its base chord) is not documented anywhere and is the part that
  is wrong.

## Unresolved questions

- Whether the guard belongs in each resolver or once in `Mode::resolve` before
  the mode dispatch. The latter is one place instead of three and cannot be
  forgotten for a future mode, which argues for it; it can be settled during
  implementation.

## Future possibilities

If `M-` chords are bound later, the guard is removed and replaced by `alt`
terms in the `match` arms, and the legend gains `M-` rows. Until then an Alt
chord is a no-op.
