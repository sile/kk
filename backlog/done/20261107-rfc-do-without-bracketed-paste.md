# RFC: Do without bracketed paste

- Status: accepted

## Summary

Stop treating a bracketed paste as an input main handles. kk asks no terminal
for the mode, reports nothing when one arrives, and drops the arm that inserted
its body. The insertion code the arm used stays, because it is the clipboard's
path too -- that is the whole of what survives from the feature.

## Motivation

bracketed paste reached kk by accident. `tuinix` 0.8.0 put the terminal in the
mode from `TerminalDriver::new()` whether or not the application asked, so a
paste arrived as one `Input::Paste` and kk grew an arm to insert it (PR #24).
kk has never called `enable_bracketed_paste()`, and since `tuinix` 0.8.1 it
cannot rely on someone else having called it either: the mode is opt-in, and
the constructor writes nothing.

The question this RFC answers is whether kk should now call it. The argument
for the mode is that a newline in pasted text becomes text rather than `Enter`,
which matters where `Enter` runs something: a shell, an editor's ex prompt, an
agent CLI. kk is not one of those. It has no command line and no way to run
anything; `Enter` at the edge of the buffer splits the line and does nothing
else. A paste of three lines arrives either as one edit that inserts two
newlines or as text, two `Enter` presses, and more text, and the buffer ends up
holding the same three lines either way.

So the mode buys kk nothing it can observe, and it is not free:

- **The failure is silent.** `tuinix`'s RFC for 0.8.1 records the reason the
  mode became opt-in: terminals (WSL2's among them) drop line breaks from a
  bracketed paste, so a pasted file arrives as one line. The decoder reports
the body as the terminal sent it, so nothing in kk can tell a corrupted paste
  from a short one. That is exactly the setup kk is run on.
- **It costs a mode on both sides.** A terminal that honours the mode sends
  every paste wrapped; kk would then have to keep the mode and the arm in
  step, and take the mode back on every way out.
- **The alternative is already correct.** Unbracketed, a paste is a burst of
  key bytes, and kk's keys already do the right thing with them. Typing a file
  in is not the same as pasting one only in how many undo steps it takes.

## Guide-level explanation

From the user's side nothing changes. A paste types its text in, at whatever
speed the terminal sends it, and a newline in the pasted text breaks the line.

```
before:  paste -> one Input::Paste -> one insertion, one undo step
         (only if tuinix's constructor put the terminal in the mode)

after:   paste -> a burst of key bytes -> typed in key by key
```

The difference a user can feel is that `C-u` after a paste takes back the last
character of it rather than the whole paste. That is the same behaviour a user
who types the text gets, and it is what every other paste path in kk -- the
clipboard's `C-y` -- does not promise either once the paste is over.

## Reference-level explanation

### The constructor does not enable the mode

kk does not call `TerminalDriver::enable_bracketed_paste()`. The call goes
nowhere: neither `App::new`, `App::from_stdin`, nor `App::build` calls it
today. Turning the mode on unasked was `tuinix`'s choice and it has been
withdrawn; kk's is to leave the terminal as it found it.

### `Input::Paste` is dropped from the edge

`App::handle_input` loses its `tuinix::Input::Paste { bytes }` arm and the
`handle_paste` method that arm called. The decoder still produces the variant,
so the match must still cover it; it stays a no-op beside the other inputs kk
does not act on, with a comment saying why: a terminal brackets a paste only
when asked, and kk does not ask.

The UTF-8 check `handle_paste` carried ("Paste is not valid text; ignored")
goes with it. A non-text paste that arrives as bytes is decoded as
`Input::Unrecognized` where the bytes are not a key, or dropped where they are
-- the same treatment as any other bytes that reach the decoder, which is all a
paste is without the mode.

### One insertion path, one caller

`State::handle_text_paste` was split out of `handle_clipboard_paste` in PR #24
so that `Input::Paste` and `C-y` could share it. With one caller the split
stops paying for itself, and the method keeps a caller-shaped name that no
longer exists. It is folded back into `handle_clipboard_paste`, along with the
argument that it took its text from a parameter instead of reading
`self.clipboard`.

### Tests

The three `State` tests for a text paste (`text_pasted_from_the_terminal_*`,
`a_terminal_paste_of_one_line_reports_its_characters`,
`a_multi_line_terminal_paste_arrives_as_one_edit`) described an event that no
longer reaches the core. They are kept where the clipboard reaches the same
code -- a multi-line clipboard paste is still one undo step, and an empty one
still says so -- and the ones that only existed to name a terminal paste are
dropped.

`tests/binding.rs` keeps its assertions that a paste never resolves to a key
binding and that `display_input` names it: both are properties of the decoder
and the binding table, and neither changes.

An end-to-end test carries the point that matters: a burst of text written
into the pty with no bracketed markers lands in the buffer. The harness writes
one key per character, so this is what the paste tests should have been: bytes
in, text out, with the terminal in no mode at all.

## Drawbacks

- A paste long enough to be worth one undo step is now as many undo steps as
  it has characters. `C-u` does not take a pasted block back.
- A paste that carries bytes which happen to decode as keys kk binds will do
  what those keys do -- `C-c` in a pasted text quits kk. Nothing in kk can
  prevent that without the mode, and a user pasting a file containing `C-c`
  is the case the mode exists for.
- `Input::Paste` joins the variants kk receives and ignores, which is a name in
the input enum with no meaning for the application that owns it.

## Rationale and alternatives

- **Call `enable_bracketed_paste()` and keep the arm.** The mode is what makes
the arm reachable, so this is the status quo with the mode made explicit. It
  takes the silent-corruption risk above to buy a coarser undo history.
- **Call the mode only on terminals that handle it.** Detecting a terminal's
  behaviour means guessing from environment variables, and the failure is
  silent, so a wrong guess is a corrupted paste nobody sees.
- **Keep the mode but refuse the paste if it looks wrong.** kk cannot tell:
  the decoder reports the body as sent, and a missing newline is a short
  paste.
- **Leave the arm as an unreachable insertion path.** It compiles, nothing
  calls it, and the next reader has to work out that it is dead. Dropping it
  is the honest version of the same decision, and the tests that covered it
  cover the clipboard too.

## Unresolved questions

None.

## Future possibilities

- If a terminal kk runs on is known to handle the mode well, the call is one
  line in `App::build` and the arm is the one this RFC removes. The reason not
  to would be the one here: the mode is worth something only where `Enter`
  runs something.
- A paste that arrives bracketed could be reported on the message line without
  being inserted ("Bracketed paste ignored"), which would make the mode's
  absence visible rather than silent. Nothing needs that today.

## Outcome

Implemented in [#25](https://github.com/sile/kk/pull/25) (merged as `5e2beb5`).

The change went in as predicted, with one part settled against the text above.
`tuinix` 0.8.1 stopped asking for the mode, and kk stops asking with it;
`State::handle_text_paste` is folded back into `handle_clipboard_paste`,
which borrows the clipboard text as a `&str` since that is all it reads.
The `Input::Paste` arm stays, because the decoder still produces the variant.

What differs is what that arm does, and it is the case listed under Future
possibilities rather than the no-op described above. Dropping the paste
silently left a terminal that brackets anyway worse off than the same bytes
without markers: a plain paste types in, and a bracketed one would have done
nothing at all with no sign that anything arrived. The arm now says `Bracketed
paste ignored` on the message line, and its comment calls it a safeguard kk
does not expect to reach rather than an ordinary path.

The tests landed as planned. The `text_pasted_*` tests call
`handle_clipboard_paste`, and `tests/e2e_paste.rs` sends `a\nb` through a new
`send_raw()` harness helper and expects `onea\nb\n` -- bytes in, text out,
with no mode on. No test asserts that a bracketed paste is ignored: that
value still reaches `handle_input`, and a test over it would pin a catch-all
arm rather than a contract.

The scope is unchanged from what is described above.
