# Bug: The search legend omits the `Enter` binding

- Status: fixed

## Summary

While a search prompt is open, `Enter` finishes the search and returns to the
edit mode (`Action::SearchFinish`). The search legend, which the crate
documents as a table kept in step with the mode's bindings, has no `Enter` row,
so the one key that ends a search is the one the legend does not name. A reader
who opens the legend to find out how to accept a hit is not told, and has to
guess or know the chord from elsewhere.

## Reproduction

Open a search prompt and look at the legend. The binding is real and resolves;
the legend just does not show it.

```text
key:  C-s             (Action::SearchEnter, the prompt opens)
key:  Esc             (Action::LegendToggle, the legend is shown)
observed: the search legend lists C-g, C-r, C-s, C-k, C-y, C-a, C-e, C-b,
          C-f, C-h, and C-d -- no Enter
          pressing Enter does run SearchFinish and leaves the prompt
expected: the search legend has a row naming Enter and its finish-the-search
          action, so every search-mode binding is discoverable from the legend
```

The binding is easy to see directly:

```rust
let key = tuinix::KeyInput {
    ctrl: false,
    alt: false,
    code: tuinix::KeyCode::Enter,
};
let resolved = kk::Mode::Search.resolve(&tuinix::Input::Key(key)).unwrap();
assert_eq!(resolved.action, Some(kk::Action::SearchFinish)); // passes
assert!(kk::Mode::Search.legend().iter().any(|row| row.contains("Enter"))); // fails
```

## Observed behavior

`resolve_search` in `src/binding.rs` binds the key on its second line:

```rust
(false, tuinix::KeyCode::Enter) => then(Action::SearchFinish, Mode::Edit),
```

but `SEARCH_LEGEND` in the same file has no row for it:

```rust
pub const SEARCH_LEGEND: &[&str] = &[
    "│C-g cancel",
    "│C-r ⇤ prev",
    "│C-s ⇥ next",
    ...
];
```

The doc on `EDIT_LEGEND`, which `SEARCH_LEGEND` follows, states the invariant
this breaks: the bindings are hard-coded, so the table is too, "and the two are
kept in step by hand". `Enter` is bound in the search mode and absent from its
table, so the two are not in step. `the_search_legend_is_exactly_this_text` in
`tests/legend.rs` pins the current, short table and so locks the omission in
place rather than catching it.

## Expected behavior

The search legend should list every binding the search mode resolves, `Enter`
included. Concretely it gains a row beside `C-g cancel`, since the two are the
two ways the prompt ends -- one abandons the search and returns the cursor to
where it started, the other accepts the search on the hit the cursor sits on:

```text
│C-g cancel
│Ent ⏎ finish
│C-r ⇤ prev
│C-s ⇥ next
```

The chord column spells the key short, `Ent`, the way the other rows shorten
theirs (`C-`, `Tab`, and the glyphs ⌫, ⌦, ⇥), and the label is a word for what
the key does, like `cancel`, `prev`, and `next` on the rows around it: `finish`
names the end of the search the way `SearchFinish` names the action. The glyph
beside the chord follows the house style. The row is the same width as the
widest existing row, so the box width does not change, and the requirement is
that the `Enter` binding appears in the table, next to `C-g`.

## Impact

An ergonomics problem, not a correctness one: the binding works, so nothing is
lost, but the legend is the editor's only discovery surface for its chords --
kk has no configuration file and no other help screen -- and a missing row is a
binding a reader cannot discover. The gap is reachable from the public API in
the sense that `Mode::Search.legend()` is a published table whose contract is
completeness. No resource effect.

## Notes

The fix is one row, `│Ent ⏎ finish`, in `SEARCH_LEGEND` in `src/binding.rs`,
plus the matching update to `the_search_legend_is_exactly_this_text` in
`tests/legend.rs`. The row is `Ent ⏎ finish`: 3 columns of chord, a space, the
glyph, a space, and the 6-column label, 13 columns before the border -- the
same as `C-k cut-tail` and `C-d ⌦ delete`, so the box width holds and the
existing padding survives. The same omission exists for `Enter` in the edit
legend, which the RFC `20260929-rfc-bind-c-j-to-newline.md` raises as an open
question rather than deciding; that one is not fixed here.
