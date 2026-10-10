# RFC: `kk` reads a pipe as the initial buffer

- Status: draft

## Summary

Teach `kk` the `less`-shaped invocation `cat file | kk`: when standard input is
not a terminal, read it to the end and open that text in a buffer with no file
behind it. The keyboard comes from the controlling terminal, so editing still
works, but the *file* commands -- save, force-save, reload -- have nothing to
act on and are dropped from the bindings and the legend while such a buffer is
open. `kk` takes `TerminalDriver::with_input` for the terminal, so nothing about
the core or the renderers changes shape. `FILE` and `--create-new` are errors
with a pipe: both name a file to open or make, and a pipe has none, so they are
turned away together as the one kind of "there is no file here" input; `--tail`
still applies, opening the piped text at its end.

## Motivation

`kk` is a writing tool, and so far it only writes *files*: `FILE` is required and
every buffer has a path behind it. To look at something that is not a file -- a
command's output, a stream, text someone pastes on stdin -- the user has to save
it somewhere first, or use a different tool. `less`, `more`, `view`, and most
pagers take the stream directly:

```console
$ mamegrep -l TODO src/ | kk
$ git show HEAD:src/main.rs | kk
$ kk < report.txt
```

Today all three fail at once. `stdin` is a pipe, so `stdin.is_terminal()` is
false, and `TerminalDriver::new()` (which requires stdin to be a terminal)
returns `STDIN is not a terminal` before `kk` draws anything. Even if it did not,
`kk` demands a `FILE` positional, so `main` would stop at that before reaching
the terminal.

The terminal side of this is already solved upstream. `tuinix` 0.7.2 added
[`TerminalDriver::with_input`], a constructor that takes an already-open terminal
descriptor as the keyboard and leaves stdout as the output; the driver's own
documentation names exactly this case (`cat f | mytui`). What is left is for `kk`
to notice the pipe, read it, open `/dev/tty` for the keyboard, and stop offering
file commands for a buffer that has no file.

[`TerminalDriver::with_input`]: https://docs.rs/tuinix/latest/tuinix/struct.TerminalDriver.html#method.with_input

## Guide-level explanation

With no pipe, `kk` behaves exactly as it does now: a `FILE` is required, and the
buffer is that file. When stdin is a pipe, `kk` reads it instead:

```console
$ git show HEAD:src/main.rs | kk
```

The editor comes up showing the output of `git show`, with the cursor at the top
and the status line reading `[STDIN:1:1]`. With `--tail` it comes up at the end
instead, the same way it would for a file. Everything that moves the cursor,
searches, marks, cuts, and pastes works the same. What is *gone* is anything that
needs a file: there is no `Tab` save, no `C-x s` force-save, and no `C-x r`
reload, because there is nowhere to save to and nothing to reload from. The legend
in the corner does not list them, and pressing their chords reports that there is
no file rather than doing nothing silently.

The buffer can still be edited, and the edits are still there to see; they just
have no way out of the process, which is the honest state of a buffer that came
from a pipe. Text that should survive is copied with `C-x w` (which writes the
text to the terminal's own clipboard) or pasted elsewhere, exactly as for a file.

`FILE` is not accepted together with a pipe. `cat f | kk g` is a mistake, and
`kk` reports it rather than guessing which of the two the user meant.

`--tail` still applies: `git show HEAD:src/main.rs | kk --tail` opens the piped
text at its end, the same way it opens a file at its end.

`--create-new` is the other file-shaped input, and it goes the way of `FILE`:
there is no file to create, so `cat f | kk --create-new` is an error rather than
a flag that quietly does nothing. It is refused for the same reason and with a
message that says so, not with a generic "unknown flag": the flag exists, and it
is the pipe that makes it meaningless.

The two shapes are one decision in `main`: is stdin a terminal?

```rust
use std::io::IsTerminal;

if std::io::stdin().is_terminal() {
    // The usual path: parse FILE and open it.
} else {
    // `cat f | kk`: read stdin, take keys from /dev/tty.
}
```

## Reference-level explanation

### Where the decision is made

`main` already parses the flags and the `FILE` positional before it constructs
`app::App`. The pipe check goes there, around that parsing, not inside `App`:
whether stdin is a terminal is a property of the process's standard input, known
once and never queried again, so it belongs where `stdin` is first looked at.

The `FILE` positional is taken with `present()` and matched on `Option`, so a
missing `FILE` is not an error by itself:

```rust
let file = noargs::arg("FILE")
    .doc("A file, optionally followed by :LINE to start at, or :LINE:COLUMN")
    .take(&mut args)
    .present();
```

Whether it is *required* is decided by the branch, in the ordinary language of
the branch rather than in the argument library:

- **stdin is a terminal**: `file` must be `Some`. `None` is the existing "missing
  FILE" error, reported by `kk`. The `--create-new` / `--tail` flags and
  `split_position` work exactly as they do today.
- **stdin is not a terminal**: `file` must be `None`. `Some` is an error ("FILE
  is given with piped input"), reported by `kk`. `--create-new` is an error for
  the same reason ("--create-new is given with piped input", or the same message
  as `FILE`): both are file-shaped input, and a pipe has no file for them to act
  on. `--tail` is *not* an error: it keeps its meaning (open at the end), applied
  to the text read from stdin.

`FILE` and `--create-new` are treated as one kind of input -- a file to open or
make -- so the piped branch turns them away together, and the terminal branch
hands them both to `App::new` as it does today. Giving `--create-new` its own
error arm rather than dropping it from the argument list is what keeps the
message specific. If `--create-new` were simply not registered when stdin is a
pipe, `noargs` would answer a piped `--create-new` with its generic "unknown
flag" text, which says nothing about *why* -- the flag is not unknown, the pipe
is what makes it meaningless. Registering it and refusing it in the branch costs
one more arm and buys the right explanation, so that is the shape to take.

This keeps the terminal/non-terminal split in one place -- a plain `if` in
`main` -- instead of threading an `Option<FILE>` through `App` and asking "is
this file-backed?" at every use. `noargs` is used up to `present()`; it is not
asked to model "required unless stdin is a pipe", which is not a property of the
argument list at all.

### The piped path

The piped branch does three things, in order:

1. Read stdin to the end: `stdin.read_to_string(&mut text)?`. A read error ends
   `kk` (there is nothing to show). Invalid UTF-8 is a read error from
   `read_to_string`, and `kk` reports it; the buffer model is text, so a non-UTF-8
   stream has no representation to fall back to.
2. Open the controlling terminal for the keyboard:
   `std::fs::File::open("/dev/tty")`. `File::open` is `O_RDONLY | O_CLOEXEC`,
   which is what `with_input` wants; the driver makes it non-blocking itself.
3. `TerminalDriver::with_input(tty)?` and hand it to `App`, along with the text
   and `None` for the path.

`--tail` folds into the same startup position machinery as the file path: the
text is the buffer, and the driver's initial position is `open_position(1, 1,
tail)` -- the buffer's end under `--tail`, the top otherwise. Nothing about the
position logic is pipe-specific; it only ever needed a buffer to apply to.

`/dev/tty` may be missing (a detached process, no controlling terminal). There
is no keyboard then, and no sensible fallback, so that is an error rather than a
silent read from stdin.

### `App` gains an optional path

`App.path` becomes `Option<PathBuf>` and `App.saved_text` becomes
`Option<String>`. `Some` is the file-backed buffer (today's behavior); `None` is
the piped buffer. The two file-touching methods (`handle_buffer_save`,
`handle_buffer_reload`) return early with a message when the path is `None`,
before any `std::fs` call:

```rust
let Some(path) = &self.path else {
    self.state.set_message("No file for this buffer");
    return Ok(());
};
```

`App::new` splits into `App::new(path, create_new, position)` (unchanged, now
delegating) and a constructor for the piped case that takes the text, the
driver, and the startup position. Both funnel into one private constructor that
fills the struct, so there is still a single place that knows the field list.
The piped constructor receives the same `open_position(1, 1, tail)` the file
path computes, so `--tail` needs no code of its own. `App::new` also builds the
`TerminalDriver` itself, as it does today; only the piped constructor is handed
one, since only it needs `with_input` over `/dev/tty`.

`render` is unchanged in shape: the status line takes a display string from the
edge (see `render_status_line`), so the piped buffer passes the constant
`"STDIN"` where the file case passes `self.path.display().to_string()` (with
`self.path` now unwrapped, which the file case can do because it is `Some`
there).

### The file commands are dropped while a pipe-backed buffer is open

Save, force-save, and reload keep their `Action` variants and their lookups in
`binding`: the binding table is keyed by chord, not by whether a file exists, and
duplicating every table for the piped case would be far more code than the three
dropped chords. Instead `App` refuses them at the edge it already owns, where the
file actually lives. The refusal sits in the two file-touching methods
(`handle_buffer_save`, `handle_buffer_reload`), which return early with a message
when the path is `None`:

```rust
let (Some(path), Some(saved_text)) = (&self.path, &self.saved_text) else {
    self.state.set_message("No file for this buffer");
    return Ok(());
};
```

Every chord that reaches the file -- `Tab`, `C-x s`, `C-x r` -- goes through one
of those two methods, so letting the methods own the message is enough to make
the refusal the same for all three. A separate `Action` match arm was considered
and is not needed: the methods already return before any `std::fs` call, so the
message has exactly one home.

### The legend hides the file rows

The legend is drawn from `Mode::legend`, a `&'static [&'static str]` fixed per
mode. Hiding rows must not mean a second hard-coded table per mode (the one that
exists is already kept in step with the bindings by hand, and a second copy is a
second thing to drift). The legend is supplied to the renderer by `App` -- it
already picks `self.mode` and passes it to `kk::render_legend` -- so the piped
case is expressed by handing the renderer a row list with the file rows removed:

- `EDIT_LEGEND`: drop the `Tab` save row.
- `EXT_LEGEND`: drop the `s   force-save` and `r   reload` rows.
- `SEARCH_LEGEND`: unchanged (it lists no file command).

The bottom border row (the one with the mode title) is part of every list and is
kept in both cases. Since every row is painted as written and the box is sized
from the rows it is given, a shorter list just makes a shorter box; the
`legend_region` / `full_legend_size` arithmetic is driven by the same list, so
nothing else has to know.

This means `render_legend` (and `legend_size`) should take the rows or a small
"which rows" value rather than a `Mode` alone, so `App` can hand over the
filtered list without the renderers learning what a pipe is. The exact split
(`Mode` plus a `hide_file_rows: bool`, a `&[&str]` parameter, or a `Mode` method
that returns the rows to draw) is an implementation choice; the invariant is
that there is exactly one list per legend and it is filtered at the edge where
"is there a file?" is known.

What landed is the `&[&str]` parameter: `render_legend(rows, frame)`,
`legend_size(rows, limit)`, and `full_legend_size(rows)`. `Mode::legend()`
still returns the full list, and the filtered lists are the two constants
`EDIT_LEGEND_NO_FILE` and `EXT_LEGEND_NO_FILE` -- the same rows with the file
commands taken out. They are kept honest by a test that derives each one from
its full list by dropping the same rows, so they are not a second table to keep
in step by hand.

### What does not change

- The core (`src/lib.rs`, `state`, `buffer`, `search_prompt`, `clipboard`) is
  untouched; none of it knows where the text came from. The two core modules
  that do change are `binding` (the two no-file row constants) and `render`
  (`render_legend` / `legend_size` now take the rows to draw), both for the
  legend only.
- The poll loop, raw mode, resize handling, mouse, and the `with_input` driver
  are all `tuinix`'s and are used as-is.
- `--create-new` and `--tail` keep their meaning on the terminal path; on the
  piped path `--tail` keeps its meaning too, while `--create-new` is refused
  alongside `FILE` as file-shaped input, with a message that names the pipe.

## Drawbacks

- A new invocation shape with its own rules (no `FILE`, no file commands), which
  is more surface than "always a file" even if each rule is small.
- `App` grows an `Option` on two fields that were plain, and every use of
  `self.path` / `self.saved_text` has to say what `None` means. The compiler
  forces the sites to be visited, but the type gets looser.
- The legend gains a second shape, and `render_legend`'s signature has to move
  off `Mode` alone. That is the cost of not having two hand-kept tables.
- The usage line still reads `Usage: kk [OPTIONS] FILE`, since `noargs` has no
  way to spell "required unless stdin is a pipe". The refusal message is what
  tells a piped user what went wrong, not the usage text.
- A pipe-backed buffer has no way to keep its edits. That is intrinsic (there is
  no path), and the message says so, but a user who edits first and thinks about
  saving second loses the text. `C-x w` (copy to the terminal clipboard) is the
  escape hatch, and the legend still lists it.

## Rationale and alternatives

- **Take `FILE` as "-" for stdin.** A conventional spelling, but it adds a
  second way to say the same thing and invites `kk -` versus `cat f | kk` to
  differ. The pipe is already unambiguous -- stdin is or is not a terminal -- so
  no flag is needed.
- **`FILE` optional everywhere, `None` means an empty unnamed buffer.** More
  than is asked for: `kk` with no arguments is not this RFC. Reading a pipe is
  the concrete request; an anonymous scratch buffer is a separate feature that
  would want its own decision about saving and about a title.
- **Keep `FILE` required, and reject the pipe.** Then `cat f | kk` cannot work
  at all, and the file commands stay. Rejected: the whole point is the pager
  shape, and the file commands are exactly what has no meaning without a file.
- **Drop `--create-new` (and `FILE`) from the argument list on the piped path.**
  Cheaper to write -- the flag is never registered, so `noargs` rejects it for
  free -- but the message is `noargs`'s generic "unknown flag", which is the
  wrong explanation. The flag is known and valid on the terminal path; it is the
  pipe that makes it meaningless, and that is what the message has to say.
  Rejected for the message, not the code: the flag is registered and refused in
  the piped branch instead.
- **Duplicate the binding tables for the no-file case.** Rejected above: it
  doubles a hand-kept table and lets the chords and the legend drift apart.
- **Refuse the file chords with "No action found" instead of a message.** That
  is what an unbound chord does today, but these chords *are* bound, and the
  reason they do not act is specific (no file). A chord that looks bound in the
  legend and then behaves as unbound is more confusing than one that is gone
  from the legend and says why.
- **Do nothing.** `kk` keeps requiring a terminal on stdin and a `FILE`, and
  reading a stream keeps needing another tool. This is the status quo the RFC
  changes.

## Impact

Not breaking: with stdin a terminal, every current invocation behaves exactly as
before, including the required `FILE` and the `--create-new` / `--tail` flags.
The new behavior only appears in an invocation that fails today. `kk`'s own
runtime dependencies do not change; it takes `tuinix` 0.7.2 or later (the release
that added `with_input`) and needs one `File::open` call, which is safe code.
`App` becomes aware of "file or not", and the legend renderer takes its rows
from the caller.

## Dependencies

The change needs `tuinix` at 0.7.2 or later for `TerminalDriver::with_input`;
`kk`'s `Cargo.toml` requirement moves to `tuinix = "0.7.2"`. No other dependency
is added or moved.

## Testing

The decision in `main` -- is stdin a terminal, and is `FILE` present -- and the
refusals in `App` are the testable parts.

- `main`'s branch was written as one: `plan_open(stdin_is_terminal, file:
  Option<&str>, create_new)` returns `Open::File` / `Open::Stdin` / an error
  message, and `main` calls it. This is where "`FILE` with a pipe is an error",
  "`--create-new` with a pipe is an error (the same refusal, not a generic
  unknown flag)", and "no `FILE` without a pipe is an error" are pinned, without
  a real terminal; a test covers each rule. `--tail` is not part of it -- it
  only says where in the buffer the cursor goes -- so it is not a parameter,
  and both branches apply it the same way.
- The no-file legends are pinned by a test that derives each `_NO_FILE` constant
  from its full list by dropping the same rows, so the two cannot drift.
- `App`'s file-command refusal is the two handlers' `None` path, which is not
  reachable from a test without a terminal; the decision it turns on ("is there
  a path?") is a field read, not a function of its own.
- An end-to-end test (`printf a | kk`) needs a PTY and the built binary; it is
  the natural test for the harness that already drives `kk` end to end, and it
  is there, not here, that the pipe actually meets the keyboard. The two error
  paths were also checked by hand against the built binary.

## Unresolved questions

- **Whether `render_status_line`'s `path` becomes `Option<&str>`.** Today the
  edge hands over a display string; the piped case hands over the constant
  `"STDIN"`, so the signature did not need to change.

The first message and the `render_legend` split were settled while implementing:
the piped buffer opens with `Read stdin`, and the legend renderers take
`&[&str]` rows.

## Future possibilities

- **An anonymous scratch buffer** (`kk` with no arguments), which shares the
  "`None` path" machinery this RFC builds and would want a save-as to give it a
  file.
- **Streaming stdin** (keeping the pipe open and appending as it arrives), which
  `less +F` does. `tuinix`'s driver not owning stdin is what makes it possible;
  the first version here reads to the end, which is the smaller, non-blocking
  step.
- **`-` for stdout** (`kk > out`), the output-side mirror. `tuinix` 0.7.2 makes
  only the input side selectable, so this would need an output-side constructor
  there first.
