# RFC: The hide-legend tests do not depend on the environment

- Status: draft

## Summary

Two tests read `KK_HIDE_LEGEND` without meaning to. The unit test that pins
`--hide-legend` calls `noargs`'s `is_present()`, which is true when the flag is
on the command line *or* when the variable is set; the end-to-end test that
starts `kk` behind a PTY lets the child inherit the variable from the test
process. A developer or CI job with `KK_HIDE_LEGEND` exported sees both fail on
assertions that never involved the variable, and the failures say nothing about
`kk`. This proposal makes the unit test assert what `kk` owns -- that
`--hide-legend` on `argv` sets the flag, leaving the variable-to-flag link to
`noargs`, which already tests it -- and gives the harness a known environment by
dropping the variable before it starts `kk`.

## Motivation

`noargs`'s `FlagSpec::take()` is the one place that decides a flag is present,
and it does so from two sources: the raw `argv` and, if the spec names one, an
environment variable (`std::env::var(name)`, non-empty wins). `Flag::is_present()`
returns `true` for `Flag::Env` and `Flag::Long`/`Flag::Short` alike, so a test
that only asks `is_present()` cannot tell which source did it.

That is fine in production -- the two spellings are meant to be one decision --
but it makes the test's outcome depend on the shell it runs in. With
`KK_HIDE_LEGEND=1` exported, `hide_legend(&["kk", "src/main.rs"])`, which passes
no flag, still reports `true`, and the test fails:

```text
assertion failed: !hide_legend(&["kk", "src/main.rs"])
```

The same variable reaches the end-to-end harness by a different road: it starts
`kk` as a child process with the usual `Command`, and a child inherits the
parent's environment unless told otherwise. With the variable exported,
`escape_toggles_the_legend` (in `tests/e2e_legend.rs`) times out waiting for the
legend it expects on startup, because `kk` came up with it already hidden:

```text
wait_until: legend visible at startup
```

Neither failure is about `kk`. The process the tests run in just happens to have
the variable set. A test whose result changes with the caller's environment is
not testing the code, it is testing the environment, and the failure it produces
sends the reader to the wrong place.

The unit test already knows this is a hazard: its own comment says "the variable
is process-wide, so a test that set it would race the other tests". It avoided
one race (setting the variable) by leaving the other (reading it) in place.

## Guide-level explanation

Nothing a user sees changes. `--hide-legend` and `KK_HIDE_LEGEND` still mean the
same thing to `kk`, and the help still shows the variable beside the flag.

What changes is what the tests claim. The unit test today answers "is the flag
present?", which folds in the environment; after the change it answers "did the
command line ask for it?", which is the part `kk` controls. The environment
mapping is `noargs`'s, and `noargs` has its own test for it (`env_flag` in
`flag.rs`, which sets the variable and expects `Flag::Env`).

The end-to-end harness changes which environment it hands its child. Today it
passes whatever the test process has; after the change it drops `KK_HIDE_LEGEND`,
so a test that wants the legend visible on startup gets it, and a test that wants
it hidden asks for it on the command line with `--hide-legend`. The harness still
inherits everything else -- `PATH`, the locale, and so on -- because those are
not part of what a test is deciding.

A developer running `cargo test` with `KK_HIDE_LEGEND` exported sees everything
pass, as they should: their shell has no bearing on whether `kk` parses
`--hide-legend`, nor on the startup state an isolated child begins in.

## Reference-level explanation

The test's helper, `hide_legend(argv)` in `src/main.rs`'s test module, currently
returns `is_present()`:

```rust
fn hide_legend(argv: &[&str]) -> bool {
    let mut args = noargs::RawArgs::new(argv.iter().map(|a| (*a).to_string()));
    hide_legend_flag().take(&mut args).is_present()
}
```

The proposal matches on the returned `Flag`'s variant instead, so the answer
distinguishes the command line from the environment. `Flag` and its variants
(`Long`, `Short`, `Env`, `None`) are public, and `take()` returns `Flag`, so the
test can name them:

```rust
fn hide_legend(argv: &[&str]) -> bool {
    let mut args = noargs::RawArgs::new(argv.iter().map(|a| (*a).to_string()));
    matches!(
        hide_legend_flag().take(&mut args),
        noargs::Flag::Long { .. } | noargs::Flag::Short { .. }
    )
}
```

The assertions keep their shape -- the flag before `FILE`, absent when not asked
for, and accepted after `FILE` -- and now mean "the command line carries the
flag" rather than "the flag is set by any means". The trailing-flag case is the
one the test was written for (a trailing `--hide-legend` must not be bound as the
path), and it is a command-line question, so matching on the variant answers it
more precisely than `is_present()` did.

The test's doc comment loses its note about the process-wide variable, which was
there to explain why the test did not set it; a test that reads no environment at
all needs no such disclaimer.

This does not add a test for the environment spelling, and that is deliberate:
the mapping is `noargs`'s, `noargs` tests it, and a test here would have to set
and unset a process-wide variable around itself, racing the rest of the suite.
The `env` name stays on the spec in `main` -- it is what puts `[env:
KK_HIDE_LEGEND]` in the help and what makes the variable work at all; only the
*assertion* narrows.

The end-to-end harness gets its known environment where it builds the child.
`KkHarness::spawn` in `tests/e2e/mod.rs` constructs the `Command` and hands it to
`termnix::Session::new`; the proposal adds one call before that:

```rust
let mut command = Command::new(kk_binary());
command.env_remove("KK_HIDE_LEGEND");
```

`Command::env_remove` unsets the variable for the child only, with no lock and no
change to the test process's own environment, so it does not have the race the
unit test's set/unset alternative has. The rest of the inherited environment is
left alone; only the one variable `kk` reads for a startup decision is cleared.
Every `KkHarness` constructor funnels through `spawn`, so `open`, `create_new`,
and `open_arg` all get the same treatment.

This is the one place the fix is behavioral rather than test-only: a test that
had been getting the legend hidden by accident now gets it visible, which is what
it always meant to assert.

## Drawbacks

- The suite stops covering the case that `KK_HIDE_LEGEND` alone is enough to hide
the legend. That behavior is real, and after this change `kk`'s own suite no
longer fails if the `env` name were dropped from the spec. The help output would
change with it, though, and `noargs` covers the mechanism, so the loss is the
link between *this* spec and *this* variable -- which is exactly the part that
cannot be tested without touching the process environment.
- The harness now edits the child's environment, so a future test that *wants*
to check how `kk` reads `KK_HIDE_LEGEND` at startup cannot do it through
`KkHarness` as it stands; it would have to set the variable back on the command
it builds. That is a fair trade for a startup state every other test can rely on,
and the spot is one line in `spawn`.
- `Flag`'s variants become part of the unit test's expectations. If `noargs`
renamed or reshaped them the test would not compile; that is a compile error, not
a silent pass, so it is recoverable.

## Rationale and alternatives

- **Match on the `Flag` variant (this proposal).** Keeps the test, keeps its
  three assertions, and removes the environment from what it reads. The variant
  is the information the helper was throwing away.
- **Set and unset `KK_HIDE_LEGEND` inside the test.** Directly tests both
  spellings. Rejected: `std::env::set_var` is process-wide, so it races every
  other test that reads the environment, and Rust's test harness runs tests in
  parallel by default. The variable is not per-test state and cannot be made so
  without a lock the whole suite respects.
- **Serialize environment-touching tests behind a mutex.** Makes the set/unset
  approach safe. Rejected for one flag: it adds a shared lock and an ordering
  rule to the whole suite so that one test can mutate global state, and `noargs`
  already owns and tests the mapping this would re-verify.
- **Drop the test and check the help text instead.** Keep only that `--help`
  prints `--hide-legend` and `[env: KK_HIDE_LEGEND]`. Rejected as a replacement:
  the help line proves the spec was written, not that `take()` reads the flag off
  `argv`; parsing a synthetic `argv` still tests more. The help text is worth
  pinning for its own sake, but it does not stand in for this.
- **Do nothing.** The tests keep failing for anyone with the variable exported,
  and the failures keep pointing at `kk` for an environment the tests chose to
  read. Rejected: the fix is two small changes, one per test shape.
- **Have the harness clear the whole environment (`env_clear`).** Start the child
  with nothing inherited at all. Rejected: `PATH` and the locale are not part of
  what these tests decide, and stripping them makes the harness depend on knowing
  everything `kk` needs to start -- more coupling than clearing one variable.
- **Have the e2e test set `--hide-legend` off explicitly, or start from wherever
  the environment puts it.** Rejected: it would have to accept either startup
  state, so it would stop pinning the one `escape_toggles_the_legend` exists to
  check. The harness dropping the variable keeps the test's expectation exact.

## Impact

Small. The unit test's helper in `src/main.rs` changes which part of the `Flag`
it looks at, and its doc comment loses a note. The e2e harness in
`tests/e2e/mod.rs` drops one variable from the child's environment. No production
code, no public API, no help text, no README.

## Dependencies

None. `noargs::Flag` is already a public type of a dependency `kk` already has.

## Testing

- `the_hide_legend_flag_is_read_from_the_command_line` keeps its three cases and
  passes with `KK_HIDE_LEGEND` set, unset, empty, and non-empty.
- `escape_toggles_the_legend` (and every other e2e case) passes with
  `KK_HIDE_LEGEND` set in the parent, because the harness drops it before
  starting the child.
- The whole suite is run both with `KK_HIDE_LEGEND=1` and with the variable
  unset, and passes either way. That is the check that the fix did what it says:
  the suite's result no longer depends on the shell.
- The case that the variable alone hides the legend is covered by `noargs`'s
  `env_flag` test, not here; this item does not add one.
- No runtime behavior changes: the help text and how `kk` reads the variable are
  untouched, so nothing a user sees moves.

## Unresolved questions

None. Whether `kk` should also pin the `[env: KK_HIDE_LEGEND]` help line is a
separate question (see "Rationale and alternatives"); this item leaves the help
as it is.

## Future possibilities

If more flags gain environment names, the same split applies: `kk` tests the
command line, `noargs` tests the variable, and no test reads the ambient
environment. Should `noargs` ever offer a way to inject the environment (rather
than reading `std::env` directly), the variable spelling could be pinned here
too without the race -- but that is a change to `noargs`, not to `kk`.
