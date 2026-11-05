# RFC: Paste from the system clipboard with `C-x y`

- Status: draft

## Summary

Bind `C-x y` to a paste that inserts the terminal's own clipboard into the
buffer, so text copied in another application can be pasted into kk without
first copying it inside kk. Today the editor can *write* the terminal's
clipboard (`C-x w`) but has no way to *read* it, and bracketed paste is not
even switched on, so the one thing a terminal would hand over for free is never
sent.

## Motivation

kk's clipboard lives in memory for the lifetime of the process
(`src/clipboard.rs`). It is filled by `C-w` and `C-x w`, and read by `C-y`. A
user who copies a URL in a browser and comes to kk to paste it has nothing to
paste: `C-y` inserts kk's own clipboard, which is empty or holds something
unrelated. The only way in is to retype the text or to open another editor.

The gap is one-sided. `C-x w` already pushes the copied text to the terminal's
clipboard through OSC 52 (`Action::MarkCopy`, `src/app.rs`), precisely so it can
be pasted *outside* kk. The reverse direction -- bringing text in from outside
-- does not exist.

The pieces to close it are already within reach, and the questions the first
draft left open have been checked against `tuinix`'s source:

- `tuinix` reports bracketed paste as `Input::Paste { bytes: Vec<u8> }` (see
  `../tuinix/src/input.rs`). The terminal wraps whatever it pastes in
  `ESC [ 200 ~` / `ESC [ 201 ~`, and the decoder holds the bytes until the
  closing marker arrives, so a paste of `"a\nb"` arrives as one value rather
  than as the keys `a`, `Enter`, `b`, and it never arrives in pieces. kk
  currently drops it: `Mode::resolve` returns `None` for any non-key input, so
  a paste is reported as `No action found`.
- Bracketed paste only reaches an application that asks for it. A terminal
  sends `ESC [ 200 ~ ... ESC [ 201 ~` when the application has put it in
  bracketed-paste mode with `ESC [ ? 2004 h`; without that, a paste is a burst
  of ordinary key bytes and is indistinguishable from typing. kk never sends
  the sequence, so today no terminal wraps a paste for it.
- `tuinix` reports a byte sequence it cannot decode as `Input::Unrecognized {
  bytes }`. `ESC ]` -- the control string OSC 52 uses -- is one of those: the
  decoder settles the whole sequence as one value and puts no bound on its
  length, which is exactly where a reply to an application's OSC 52 query would
  surface.
- `TerminalDriver` implements `std::io::Write`, and its `Drop` takes back only
  the mouse reporting, the alternate screen, raw mode, and the cursor. It has
  no method for bracketed paste or for an OSC 52 query, so both sequences are
  written by kk through `write_all` and taken back by kk before the driver
  drops.

So the work is not new plumbing but switching on the one mode that makes a
paste identifiable, recognising the input kk already receives, and deciding
what it means.

## Guide-level explanation

The user copies text somewhere else and focuses kk. From there, two things put
the text in, and which one happens depends on the terminal rather than on the
user:

- **The terminal pastes it.** Most terminals turn a paste gesture (mouse middle
  click, `C-Shift-V`, a menu item) into a bracketed paste. kk has bracketed
  paste switched on, so it receives the text as one value and inserts it.
- **The user types `C-x y`.** kk asks the terminal for its clipboard (OSC 52)
  and inserts what comes back.

Either way the text lands at the cursor as if typed, and a multi-line paste is
inserted as several lines: it does not run one `Enter` per newline, so it
cannot be mistaken for commands, and the whole paste is one undo step.

The first needs no key from kk's side, which is why the second exists: a paste
that arrives unprompted is a courtesy the terminal offers, while `C-x y` is
the command that asks for one on a terminal that does not offer it in the way
the user pressed.

`C-x y` is an extension-mode chord. Its neighbours are `C-x r` (reload), `C-x
s` (force-save), `C-x w` (copy), `C-x a` and `C-x e` (buffer start and end) --
the chords that act on the buffer as a whole, which is what reading another
application's clipboard is. `C-v` was the first draft's choice and is dropped:
it is many users' "insert verbatim" key, and a chord under `C-x` is where this
editor already keeps the commands that reach outside the buffer.

This mirrors the existing behaviour of `C-y` (paste kk's clipboard) but reads
the terminal instead. Like `C-y`, it does not add to kk's own clipboard: after
pasting, `C-y` still inserts what `C-w`/`C-x w` last put there, because the two
clipboards stay separate.

## Reference-level explanation

### Binding

`C-x y` is unbound today. Add it to the extension mode:

- `src/binding.rs`, `resolve_ext`: `(false, KeyCode::Char('y')) =>
  then(Action::ClipboardPasteFromTerminal, Mode::Edit)`, beside `C-x w`.

The extension mode's own chords drop the `C-x` prefix, so a bare `y` is the
whole chord after `C-x`, exactly like `s`, `r`, `w`, `a`, and `e`.

It stays in the extension mode rather than the edit mode because every path it
raises is a question answered by the terminal, and a query that is never
answered leaves a message behind. It also leaves `C-y` alone, which keeps kk's
clipboard and the terminal's apart by source as well as by key.

The search prompt does not get the chord. A paste into a query is a rarer need
than a paste into a buffer, and the prompt already takes `C-y` for kk's own
clipboard; adding a second paste chord there would be a second meaning for the
same gesture.

### Bracketed paste is switched on

kk never sends `ESC [ ? 2004 h`, so no terminal wraps a paste for it. It has to
start doing so, and has to take the mode back when it leaves:

- `App` writes `\x1b[?2004h` once the driver has taken over the terminal, the
  same way `TerminalDriver::new` writes the alternate-screen sequence.
- Leaving kk writes `\x1b[?2004l`. `TerminalDriver`'s `Drop` restores the
  mouse, the alternate screen, raw mode, and the cursor, but knows nothing
  about bracketed paste, so kk restores it itself: `App::run` writes the
  sequence when its loop ends, and `App`'s own `Drop` covers the paths that
  leave through `?` without reaching there. The two are written so that the
  sequence goes out exactly once on an ordinary exit.
- A terminal that does not understand the mode ignores it and keeps sending
  plain bytes, which is what kk does today.

`Input::Paste` is then handled in `App::handle_input` beside the mouse arm,
before `mode.resolve`, and inserts the bytes as text. This keeps the binding
table about keys, lets a paste mean one thing in every mode, and mirrors how
mouse input is already resolved outside the table.

The body is `Vec<u8>` because the encoder does not insist it is UTF-8. kk does:
a body that is not valid UTF-8 is reported rather than inserted, following the
existing `Unrecognized` contract rather than inventing a lossy conversion.

### One insertion path for both pastes

The bytes are inserted by the same code `C-y` uses, so the newline splitting
exists once. `State::handle_clipboard_paste` currently reads kk's clipboard
from inside `State`. The refactor is to split it into a method that inserts a
`&str` and a thin caller that reads kk's clipboard and passes it, so the edge
can hand `Input::Paste`'s text to the same method. The search prompt's paste
reads `search_clipboard`, so it keeps its own thin caller over the same
insertion method.

### OSC 52 query

The query needs three things, and none of them needs a `tuinix` change:

1. **Writing the query.** `TerminalDriver` has no method for it, but it
   implements `std::io::Write`, so the query is
   `driver.write_all(b"\x1b]52;c;?\x07")` followed by a flush. No `tuinix`
   change is needed, so this RFC is not blocked.
2. **Reading the reply.** The reply arrives as `Input::Unrecognized { bytes }`
   holding `ESC ] 52 ; c ; <base64> BEL` (or `ST`-terminated). kk decodes the
   base64 to UTF-8 and inserts it.
3. **Giving up.** The reply may never come. kk must not block: it sends the
   query and continues; if no reply is decoded before the next input, the paste
   reports `Could not read the clipboard` rather than waiting.

This is the fragile part. OSC 52 reads are answered by few terminals (they are
often disabled for security, since the same channel lets a program read the
clipboard). That is the reason bracketed paste is the preferred path and OSC 52
is only the fallback.

### A one-shot reply is not `Unrecognized` forever

`Input::Unrecognized` is also how the decoder reports anything it cannot parse.
kk must not treat *any* `Unrecognized` as a clipboard reply: it parses the bytes
and only acts when they are exactly an OSC 52 reply it asked for. Stray
`Unrecognized` input keeps its current behaviour (report and ignore).

### What a paste does not carry

A bracketed paste says nothing about *where* the text came from. A terminal
wraps a mouse paste, a `C-Shift-V`, and a menu paste the same way, and on a
terminal whose clipboard is kk's own (one that honoured the `C-x w` OSC 52
write) the text may be exactly what `C-y` would insert. kk therefore does not
label a paste by source; it inserts the text, which is what the gesture asked
for either way.

## Drawbacks

- Most terminals answer neither a bracketed paste from a bare key nor OSC 52
  reads, so on many setups `C-x y` reports that the clipboard could not be
  read. The bracketed path is what actually carries most of the feature, and it
  needs no key at all.
- OSC 52 reads are a security-sensitive channel. Asking for them in kk is fine,
  but users on terminals that answer must know a pasted string came from the
  system clipboard unchanged.
- Switching bracketed paste on changes what kk receives on terminals that
  support it: a paste that used to arrive as a burst of keys now arrives as one
  value. That is the point, but it is a change in kk's input, not only in its
  output.
- The refactor of `handle_clipboard_paste` to share the insertion path touches
  existing, working code for the benefit of the new command.

## Rationale and alternatives

- **Do nothing.** The user retypes or copies inside kk. Works today, but is the
  exact inconvenience the feature removes.
- **Bind `C-y` to read the terminal clipboard instead of kk's.** Rejected: kk's
  clipboard is the one `C-w` fills and is what a cut-then-paste expects; making
  `C-y` depend on the terminal would break a working flow.
- **Bind `C-v` in the edit mode, as the first draft proposed.** Rejected: `C-v`
  is "insert verbatim" in readline and emacs, and every other command that
  reaches outside the buffer already sits under `C-x`. A chord that produces a
  message only when the terminal answers is better kept where the deliberate
  buffer-level commands live.
- **Only bracketed paste, no `C-x y`.** If the terminal already turns a paste
  gesture into a bracketed paste, no binding is needed -- but only where the
  terminal offers the gesture, and a reader who expects a key finds nothing.
  Supporting the key as a query trigger covers the setups bracketed paste does
  not.
- **Only OSC 52, no bracketed paste.** Simpler in code but fails on the many
  terminals that ignore the query, and a paste the terminal sends anyway would
  still arrive as a burst of keys.
- **Depend on a clipboard crate.** Pulls in system clipboard libraries, which
  assume an X11/Wayland session and undo kk's "minimum dependencies" stance.
  Out of scope; the terminal is the only clipboard kk should touch.

## Unresolved questions

- **The name of the action.** `ClipboardPasteFromTerminal` is long.
  Alternatives: reuse `ClipboardPaste` with a source argument, or
  `PasteFromSystem`.
- **Timeout behaviour.** How long, if at all, does kk wait for an OSC 52 reply
  before reporting failure? "Until the next input" is the current strawman but
  is not measured against a real terminal.
- **What `C-x y` does when a bracketed paste arrives instead.** A terminal may
  turn the chord itself into a paste, so kk receives `Input::Paste` and never
  sees the key. That works, but the message line then says what a paste always
  says rather than what the chord asked for. To be settled when the message is
  written.

## Future possibilities

- If the terminal clipboard read works, `C-y` could be taught to fall back to
  it when kk's own clipboard is empty, making cut-then-paste and copy-then-paste
  behave the same way.
- If the terminal clipboard read works in the search prompt's message line, it
  could gain the chord too, pasting into the query.
- The extension mode's legend has five rows; `y   paste` would make it six, and
  the legend's height rule already drops a row when the frame is short. Whether
  the row is worth its height is worth measuring on a real terminal.
- A paste that arrives as `Input::Paste` and fails to decode as UTF-8 could be
  reported with the raw bytes, following the existing `Unrecognized` contract.
