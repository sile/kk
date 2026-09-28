# RFC: Save on `Tab`, force-save on `C-x s`

- Status: accepted

## Summary

In the extension mode, save and force-save are told apart by case: bare `s`
saves, bare `S` force-saves. This proposal moves the pair onto flat, more
reachable chords -- `Tab` saves and `C-x s` force-saves -- and removes the
capital-`S` binding that case currently uses. Force-save keeps a whole chord
because it is the command that discards whatever is on disk; plain save shrinks
to the one key a reader reaches for without thinking.

## Motivation

Saving is the most common reason to leave the edit mode, and today it costs two
chords: `C-x` to open the extension mode, then `s`. That is two keystrokes and a
mode switch for the command a user runs after nearly every edit.

Worse, the two save commands are one shift apart:

```text
C-x s   save
C-x S   force-save
```

`S` is a single fat-fingered capital away from silently overwriting a file that
changed on disk -- the one command in the editor that can throw data away. The
[RFC that put the `M-` commands behind `C-x`](20260928-rfc-alt-modified-keys-match-nothing.md)
kept case as the discriminator, and the extension legend documents it as
`S   force-save`. But a destructive command and a safe one should not be told
apart by an accidental shift.

The edit mode is also where the user's hands already are. `Tab` is free there
(the edit mode binds no `Tab`; in the search prompt `Tab` already means "next
hit", which is the same idea of "move on"), and it sits under the left pinky
with no modifier, so save becomes one press with no mode switch:

```text
Tab     save
```

and the destructive sibling is pushed out to a full chord:

```text
C-x s   force-save
```

## Guide-level explanation

After this change the bindings read:

| Chord    | Command    |
| -------- | ---------- |
| `Tab`    | save       |
| `C-x s`  | force-save |

`Tab` saves the buffer to its file and reports `Saved!`. It works from the edit
mode directly, without opening the extension mode first. When the file on disk
no longer holds what the edge last read or wrote, the save is refused, exactly
as today; the message explains the conflict and the user reaches for
force-save.

Force-save is no longer a stray capital. It is `C-x` then `s`, the same shape as
every other extension chord. A user who wants to write over a file that changed
elsewhere now has to mean it: open the extension mode, then press `s`.

The capital-`S` binding is gone. In the extension mode `S` does nothing, the
same as any other unbound key.

## Reference-level explanation

In `src/binding.rs` the edit mode gains one arm:

```rust
(false, tuinix::KeyCode::Tab) => act(Action::BufferSave),
```

The extension mode's two arms collapse onto the lower-case key, and the
capital-`S` arm is deleted:

```rust
// before
(false, tuinix::KeyCode::Char('S')) => then(Action::BufferForceSave, Mode::Edit),
(false, tuinix::KeyCode::Char('s')) => then(Action::BufferSave, Mode::Edit),

// after
(false, tuinix::KeyCode::Char('s')) => then(Action::BufferForceSave, Mode::Edit),
```

Three things are load-bearing:

- **`Tab` is a plain edit-mode binding, so the mode does not change.** The edit
  mode's `Action::BufferSave` arm behaves as it does from the extension mode's
  today: the action itself has no mode in it, and `Resolved::mode` is `None`, so
  the editor stays in the edit mode. The comment on the extension resolver that
  says it drops the ctrl prefix and uses case to separate save from force-save
  must be rewritten to match; nothing else in that resolver's shape changes.
- **`C-x s` is now force-save, so the extension mode no longer has a
  non-destructive save.** A user who opens the extension mode (`C-x`) and presses
  `s` force-saves. That is intended -- the chord is the deliberate one -- but the
  extension legend has to say so (`s   force-save`), and anyone who learned
  `C-x s` as the safe save must relearn it as `Tab`.
- **`Tab` in the search prompt is untouched.** Search already binds
  `(false, KeyCode::Tab) => Action::SearchNextHit`; the new edit-mode arm is a
  different resolver and does not interfere. The two modes disagree on what
  `Tab` means (save vs. next hit), which is fine: they are different modes, and
  it mirrors how they already disagree on every other shared key.

The legend tables are `const` and hard-coded, kept in step with the resolvers by
hand, so both change in the same commit:

```rust
// EDIT_LEGEND: the new binding goes next to the other edit-mode command rows
"\u{2502}Tab save",

// EXT_LEGEND: `s` is now the destructive one and the capital is gone
"\u{2502}s   force-save",
```

The `EXT_LEGEND` still lists `C-g cancel`, `s force-save`, `r reload`, `a bof`,
`e eof`; only the `S` row is removed and the `s` label changes. `legend_size`
for `Mode::Ext` does not change (the removed row is not the widest). The
`EDIT_LEGEND` grows by one row and may grow in width, which the size assertions
in `tests/legend.rs` pin.

No new `Action` is needed: `Action::BufferSave` and `Action::BufferForceSave`
already exist and their meanings do not change. Only which chord reaches them
does.

## Drawbacks

- **`C-x s` changes meaning.** It has been the safe save since the extension
  mode was introduced; anyone with the old binding in muscle memory will
  force-save where they meant to save. This is the cost of making the safe
  command reachable in one key and is accepted.
- **`Tab` is not a "save" key by convention.** It inserts a tab, or indents, in
  most editors; here it saves. The edit mode never bound it and the buffer has
  no tab-indent command, so nothing is lost, but a new user may press `Tab`
  expecting to indent and save the file instead.
- **The extension mode loses its safe save.** Its legend now offers only the
  destructive write, so a user who is used to "open `C-x` to do file things"
  finds the safe one moved out from under them.
- **One more edit-mode binding to keep in sync by hand.** The legend and the
  resolver are already duplicated tables; this adds a row to both.

## Rationale and alternatives

- **Do nothing.** The current bindings work and are documented. Rejected because
  the safe and destructive saves are one stray shift apart, and because the most
  common command is buried behind a mode switch.
- **`C-s` for save instead of `Tab`.** `C-s` is the conventional save chord
  everywhere else. It is rejected here: `C-s` is already the search entry
  (`Action::SearchEnter`), and taking it would mean moving search -- a much
  larger change than this one. `Tab` is free and reachable.
- **`C-x C-s` for save, keeping `C-x s` safe.** The Emacs shape, and it would
  keep the extension mode's safe save. Rejected because it is *less* reachable
  than today, not more, and the goal is to make the common save cheap.
- **Keep `S` and add `Tab`, not promoting force-save.** `Tab` saves, `C-x s`
  saves, `C-x S` force-saves. Rejected: it leaves the destructive command one
  shift from a safe one, which is the problem being fixed.
- **`C-x s` for save and `Tab` for force-save.** Puts the dangerous command on
  the most reachable key. Rejected outright.
- **Ask for confirmation on force-save.** Would blunt the accidental-shift risk
  without moving any key, but kk has no prompt for it and the mode switch to
  reach `C-x` is already a deliberate act. Kept as a future possibility.

## Unresolved questions

- Whether `Tab` should also save from the search prompt, where it currently
  means "next hit". This proposal leaves the prompt alone; the edit mode is
  where the buffer is edited, so it is where save belongs.
- Whether removing `S` should leave it unbound or rebind it to plain save as a
  courtesy for muscle memory. Unbound is proposed, since a silent alias would
  keep the accidental-shift hazard in a different shape.
- The exact wording and column alignment of the `Tab save` legend row (the edit
  legend aligns its labels with padding by hand).

## Future possibilities

If kk ever grows an indent or tab-insert command, `Tab` would be the natural
key for it and save would need another home; pinning `Tab` to save now trades
that away, and this RFC is the place to revisit it.

A confirmation prompt on force-save ("file changed on disk, overwrite?") would
let the safe and destructive saves share more of the reachable key space, and
would compose with this change rather than replace it.

## Outcome

Implemented in [#3](https://github.com/sile/kk/pull/3) (merged as `3c489c7`).

Bindings moved as proposed: `Tab` saves from the edit mode without a mode switch,
`C-x s` force-saves, and the capital-`S` arm is gone. Two details the RFC left
open were resolved during the work.

The saved-file-changed messages in `src/app.rs` (`Changed on disk; C-x S to
overwrite` and `File is gone; C-x S to overwrite`) still named the removed
binding, so both now point at `C-x s`. The RFC did not mention them.

The `Mode::legend_size` doc example pinned the extension legend at 7 rows; with
the `S` row removed it is 6, and the doctest caught the stale count. The `Tab`
row also gained an icon, `\u{21e5}`, matching the arrows the search legend uses
(`C-r \u{21e4} prev`, `C-s \u{21e5} next`), which settled the question of the
row's exact wording and alignment left under "Unresolved questions".

Tests cover the new arms in `tests/binding.rs` and `tests/e2e_save.rs`, and
`tests/legend.rs` pins the new row text and the shrunk extension legend. No new
`Action` was needed and the extension resolver's shape is unchanged apart from
losing the capital-`S` arm. The search prompt's `Tab` still means "next hit".

The scope is unchanged from what is described above.
