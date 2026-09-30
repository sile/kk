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

`Mode::resolve` returns `None` for a key whose `alt` is true, after it has
checked that the input is a key and before it dispatches on the mode:

```rust
pub fn resolve(self, input: &tuinix::Input) -> Option<Resolved> {
    let key = match input {
        tuinix::Input::Key(key) => key,
        _ => return None,
    };
    if key.alt {
        return None;
    }

    match self {
        Mode::Edit => resolve_edit(key),
        Mode::Search => resolve_search(key),
        Mode::Ext => resolve_ext(key),
    }
}
```

The guard goes here rather than in each of `resolve_edit`, `resolve_search`,
and `resolve_ext` because it is one place instead of three, and because a mode
added later cannot forget it: every mode reaches `resolve`, while a new
`resolve_*` is only reached through this dispatch. Nothing else changes:
`ctrl` and `code` still select the arm, and a key with `alt == false` resolves
exactly as it does today.

Since the guard is on the `alt` field alone, it also drops the chords that
today resolve through a `ctrl`-free arm by accident. `M-<UP>` currently matches
`(false, KeyCode::Up)` and moves the cursor; after this change it is a no-op
like every other Alt chord. That is the intended reach of the rule — an
unbound chord does nothing — but it is broader than the accidental `M-a` edit
that motivates it, and is listed in Drawbacks.

`fmt::display_input` already renders an Alt chord as the chord without Alt
(`src/fmt.rs`: "Alt is a modifier `kk` does not act on, so it is not part of the
spelling"). With this change the rendering and the resolution agree: an Alt
chord both looks and resolves like nothing. The `src/fmt.rs` comment should say
so: it currently explains the rendering alone, and reads as if an Alt chord were
handled, rather than inert.

## Drawbacks

- The guard is behavior-visible for every Alt chord, not only the accidental
  ones. `M-<UP>` and the other Alt-modified keys that today fall through to a
  `ctrl`-free arm stop working. No such chord is documented or bound on purpose,
  so nothing a user was told to do is lost, but the change reaches further than
  the `M-a` edit that motivates it and should be called out.
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

None. The guard belongs in `Mode::resolve`, before the mode dispatch, as shown
in the reference-level explanation: one place instead of three, on the path
every mode takes, and out of the resolver functions that only select an arm.

## Future possibilities

If `M-` chords are bound later, the guard is removed and replaced by `alt`
terms in the `match` arms, and the legend gains `M-` rows. Until then an Alt
chord is a no-op.
