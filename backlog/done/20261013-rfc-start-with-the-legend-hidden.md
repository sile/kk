# RFC: The legend can start hidden

- Status: accepted

## Summary

Give the key-binding legend a starting state the user can set: `--hide-legend`
on the command line, or `KK_HIDE_LEGEND` in the environment, makes `kk` come up
with the legend hidden. It is the same state `Esc` already reaches, so `Esc`
still toggles it back on; nothing about the legend itself, the bindings, or the
`Esc` chord changes. The two spellings set one bit, so the flag and the variable
are one `noargs` flag spec with an `env` name rather than two settings to keep
in step.

## Motivation

`kk` always draws the legend in the top-right corner, and it takes a `Esc` to
get rid of it. A user who already knows the chords -- or who simply wants the
corner for text -- presses `Esc` at the start of every session, which is one
keystroke a startup setting can spend for them.

Two shapes of that user exist, and they want different spellings:

```console
$ kk --hide-legend src/main.rs    # this run
export KK_HIDE_LEGEND=1           # every run, for a shell profile
$ kk src/main.rs
```

The flag is for the invocation; the variable is for the habit. Neither exists
today, and `kk` reads no environment variable at all -- so the variable also
decides how `kk` names them.

## Guide-level explanation

By default nothing changes: the legend is drawn in the corner, and `Esc` hides
it. With `--hide-legend`, or with `KK_HIDE_LEGEND` set to any non-empty value,
`kk` starts with the legend hidden:

```console
$ kk --hide-legend src/main.rs
$ KK_HIDE_LEGEND=1 kk src/main.rs
```

The corner is empty and the text is unobstructed. The legend is not gone, just
not shown: `Esc` brings it back, and `Esc` again hides it, exactly as before.
The status line, the message line, the chords, and the legend's own contents are
unchanged; only which state `kk` starts in is.

Nothing else about the two spellings is new. `--hide-legend` is an ordinary
flag, so it may appear before or after the `FILE` in the usual way, and it may
be combined with `--tail` and `--create-new`. It has no bearing on the piped
shape: `git show HEAD:src/main.rs | kk --hide-legend` reads the pipe and starts
with the legend hidden, just as `kk --hide-legend src/main.rs` opens the file
that way.

## Reference-level explanation

### Names

Two names have to be chosen, and they are one decision: the variable is spelled
the same way as the flag, under a `KK_` prefix.

**Flag: `--hide-legend`.** What the setting does is reachable again from inside
the session: `Esc` brings the legend back. A flag named for that action -- the
same action the user would otherwise take -- names something the user can undo,
and reads as the shortcut it is:

```console
$ kk --hide-legend src/main.rs     # instead of pressing Esc at startup
```

- `--no-legend` says the legend is *not* there, which sounds like a property of
the run rather than an action taken at startup. It also has no inverse to offer:
`--legend` would be the no-op spelling of the default, so the pair `--legend` /
`--no-legend` cannot be completed the way `--hide` / `--show` can.
- `--legend=off` spells a boolean as a value, which is more syntax (a value, its
  parsing, its error case) than one bit deserves, and it sits oddly beside the
  plain `--tail` and `--create-new`.
- `--hide-legend` makes the default the absence of a flag, as with every other
  flag here, and it does so while still naming the action the flag stands for.

There is deliberately **no short form.** The obvious letter is `-l`, and it
would read two ways at once: `-l` looks like it *turns the legend on* (the
initial of "legend") while actually turning it off. `-L` avoids that but is
still one letter standing for a two-letter idea, and this is a set-once setting
rather than something typed per run. `-c` and `-t` are the only shorts today
because those two *are* typed per run; a flag that lives in a shell profile can
be spelled out, and the long form is what the help text and a reader's memory
have to work with anyway.

**Variable: `KK_HIDE_LEGEND`.** The word-for-word spelling of the flag, in an
upper-cased `KK_` namespace:

- Reusing the flag's words (`HIDE_LEGEND`) rather than inventing a second name
  (`KK_LEGEND=0`) means there is one name to know, and a reader who has met the
  flag can guess the variable and be right.
- The `KK_` prefix is `kk`'s namespace to use. An unprefixed `HIDE_LEGEND` would
  sit in the same flat space as conventions like `NO_COLOR` and claim a name
  that is not `kk`'s to claim; `KK_` cannot collide with another program's
  variable and cannot be mistaken for a universal one.
- `KK_HIDE_LEGEND` is read as a *presence*, not a value: any non-empty setting
  hides the legend at startup, and an unset or empty variable leaves it alone.
  This is the convention the name already implies (the setting exists or it does
  not), and it is the rule `noargs` applies to a flag's `env` name, so the two
  cost one spec rather than two.

`KK_HIDE_LEGEND=0` therefore still hides the legend: a variable whose presence
is its meaning has no "off", and any value is a way of writing "set". This is
standard for this kind of variable (`NO_COLOR=0` does not re-enable color) and is
what keeps the flag and the variable one setting.

### Where the decision is made

The flag is registered in `main` beside the ones already there, and is taken
before the `FILE` positional for the same reason as `--create-new` and `--tail`:
a trailing `--hide-legend` must be read as a flag, not bound as the path. The
spec is named rather than built inline, so that its registration -- the part
`kk` owns, the flag being present -- can be tested:

```rust
fn hide_legend_flag() -> noargs::FlagSpec {
    noargs::flag("hide-legend")
        .doc("Start with the key-binding legend hidden")
        .env("KK_HIDE_LEGEND")
}

// in `main`:
let hide_legend = hide_legend_flag().take(&mut args).is_present();
```

The `.env("KK_HIDE_LEGEND")` is the whole of the variable's support: it makes the
flag present when the variable is set and non-empty, in the same call that
reads the flag, so `is_present()` is already "the flag or the variable" and no
merge of our own is written. The flag is not part of `plan_open`'s decision:
it is not file-shaped input, so it is orthogonal to whether stdin is a pipe and
is passed on to whichever constructor the branch takes.

### The initial state reaches `App`

`App.legend_visible` is `true` in `App::build` today. It becomes a parameter of
the constructors, so the field is still filled in the one place that knows the
field list:

- `App::new(path, create_new, position, legend_visible)`
- `App::from_stdin(text, driver, position, legend_visible)`

The parameter is named for the field, `legend_visible`, rather than for the
flag: it is the field's *value*, so with `--hide-legend` the constructors are
called with `false`. The flag and the field differ in polarity (`hide` against
`visible`) and the negation happens once, at the call in `main`, where the
flag's name is in view.

`build` takes it and stores it, and `main` hands the same value to both branches.
The alternative -- an `App::set_legend_visible(false)` call after construction --
was rejected: it puts the initial value at the call site rather than in the
constructor, so a new constructor would compile while silently keeping the
legend on, and a caller would have to remember the call.

Threading one more `bool` also raises whether the parameters should be grouped
into a startup struct (`Startup { position, legend_visible }`). Not done here:
one more parameter is still cheaper to read than a struct, and the struct earns
its place when a third startup bit appears, not before.

Nothing else in `App` changes: `legend_visible` is already the field the `Esc`
toggle flips and the renderer already checks, so both the toggle and the
"step aside for the cursor" test keep working against the new starting value.

### What does not change

- The legend, its rows, `render_legend`, and `legend_size` are untouched; the
  new bit is read where `legend_visible` is already read.
- `Esc` and `Action::LegendToggle` are untouched. The setting is a starting
  state, not a binding, so a session that begins hidden can be toggled on and
  off as usual.
- The bindings, the modes, and the piped shape are untouched. `--hide-legend`
  is accepted on both the file and the pipe path.
- Startup messages ("Opened", "Created", "Read stdin") are unchanged: a hidden
  legend is not announced, since it is the state the user asked for.
- `kk` still reads exactly one environment variable, the one the flag names.

## Drawbacks

- `kk` gains its first environment variable, and with it the general question of
  which settings deserve one. The answer here is "a setting that is chosen once
  and then kept", which keeps the door closed on per-run settings being spelled
  twice.
- A startup default can surprise a reader of someone else's terminal ("why is
  the legend not drawn?"). The `Esc` toggle is the mitigation, and the flag is
  visible in `--help`.
- One more parameter on the two constructors, and one more thing for `main` to
  pass down both branches.
- A presence-only variable means `KK_HIDE_LEGEND=0` hides the legend, which a
  user may read as a bug. It is the convention for this kind of variable, but
  it does have to be documented rather than guessed.
- `Esc` hides the legend too, and the flag is a startup way of reaching the same
  state. The two are not one command: `Esc` toggles, the flag sets the starting
  state. That is a small asymmetry in a program that values having one way to do
  a thing.

## Rationale and alternatives

- **Do nothing.** `Esc` at startup is one keystroke; the setting saves it for
  the user who presses it every time. Rejected because that user is the one the
  setting exists for, and the cost is one flag and one boolean.
- **A configuration file.** Already listed as unsupported in `README.md`, and
  far more machinery than one bit. The environment is the smallest place a
  default can live without one.
- **Only the flag, no variable.** The flag cannot be put in a shell profile, so
  the habitual user would still type it every run -- which is the problem being
  solved, moved from `Esc` to a flag.
- **Only the variable, no flag.** A variable is a poor fit for a single run
  (`KK_HIDE_LEGEND=1 kk f` is longer than `kk f` plus `Esc`), and it is invisible
  in `--help`.
- **A positive variable (`KK_LEGEND=0`).** Turns the name into a value to parse,
  with "off", "0", "no", and "false" to define, and puts the variable and the
  flag at odds in spelling. Presence is the smaller rule.
- **`--no-legend`.** Rejected above: it names a state, not the action the user
  would otherwise take, and leaves `--legend` as a no-op to complete the pair.
- **`--legend=on|off`.** Rejected above: a value for one bit.
- **A short form `-h`-style letter for the legend.** Rejected above: `-l` reads
  as "legend on", and this is a set-once setting rather than something typed per
  run.
- **An unprefixed `HIDE_LEGEND`.** Rejected above: not `kk`'s namespace.
- **A `set_legend_visible` call instead of a constructor parameter.** Rejected
  above: it moves the initial value out of the constructor.

## Impact

Not breaking: with no flag and no variable, every current invocation behaves
exactly as before, including the legend being drawn and `Esc` hiding it. The new
behavior appears only when asked for. No dependency is added or moved; the
environment read is `noargs`'s `FlagSpec::env`, already used by the flag library
the binary parses with. `App`'s two constructors take one more parameter, and
`main` reads one more flag.

## Testing

- `main` parses the flag, and `noargs` turns the identical value on for the
  environment name; the flag/env merge is `noargs`'s `is_present()`, so the
  testable part of our code is the argument registration and the value reaching
  `App`.
- A test parses a synthetic `argv` through `hide_legend_flag()` and pins that
  `--hide-legend` makes the flag present, and that it is still a flag when it
  trails the `FILE`. The test takes `KK_HIDE_LEGEND` unset. Whether the variable
  also makes it present is `noargs`'s rule and is covered by its own tests;
  pinning it here would mean an `unsafe` `std::env::set_var` in the test process,
  which races with the other tests running in parallel (the variable is
  process-wide), so `kk` does not do that for a rule it does not own.
- `App`'s starting `legend_visible` is a constructor argument and is not
  reachable from a test without a terminal, so it is not unit-tested; the value
  is passed through `build` unchanged, which the type makes hard to get wrong.

## Unresolved questions

- **Whether the `--help` block in `README.md` is updated in the same change.**
  The block already omitted `--tail` (it showed only `--version`, `--help`, and
  `--create-new`), so it was stale independently of this RFC. Settled while
  implementing: the stale `--tail` row was added first, on its own, and this
  flag's row was added here, so the two are two commits and the block now shows
  every flag.
- **Whether `--hide-legend` should also be settable per invocation *and* per
  session by the same name**, which is what is proposed (one name, two places).
  If a future setting wants a different spelling per scope, the one-name rule
  would have to be revisited.

## Future possibilities

- **More startup defaults** (a starting mode, a save-on-quit prompt, a preferred
  viewport position), which would make the startup struct worth introducing and
  give the `KK_` namespace more than one member.
- **An `--hide` / `--show` pair** for other always-on decorations, if `kk` grows
  any (`README.md` currently lists none), which the flag's wording already leaves
  room for.
- **`--show-legend` as an explicit "on"** if a future default ever changes to
  hidden; not worth having while hidden is the narrower request.

## Outcome

Implemented in [#19](https://github.com/sile/kk/pull/19) (merged as `60ea683`).

`--hide-legend` and `KK_HIDE_LEGEND` are one `noargs` flag spec, the variable
name given to `env`, so the flag is present when the variable is set to anything
non-empty. The flag is taken beside `--create-new` and `--tail`, before the
`FILE` positional; both `App::new` and `App::from_stdin` take the legend's
starting state as one more argument and the shared `build` stores it, with the
one negation from the flag's `hide` to the field's `visible` happening in `main`.

The flag's spec is a named `hide_legend_flag()` rather than built inline, so its
registration can be tested by parsing a synthetic `argv`; that is the one test
added. The `--help` block in `README.md` gained the row and the column width it
needed. The `--help` block also turned out to be missing `--tail`, which was
added first, on its own.

The scope is unchanged from what is described above.
