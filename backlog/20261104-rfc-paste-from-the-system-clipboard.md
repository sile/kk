# RFC: Paste from the system clipboard with `C-v`

- Status: draft

## Summary

Bind `C-v` to a paste that inserts the terminal's own clipboard into the
buffer, so text copied in another application can be pasted into kk without
first copying it inside kk. Today the editor can *write* the terminal's
clipboard (`C-x w`) but has no way to *read* it, and the bracketed paste the
terminal already sends is thrown away.

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

The pieces to close it are already within reach:

- `tuinix` reports bracketed paste as `Input::Paste { bytes }` (see
  `../tuinix/src/input.rs`). The terminal wraps whatever it pastes in
  `ESC [ 200 ~` / `ESC [ 201 ~`, so a paste of `"a\nb"` arrives as one value
  rather than as the keys `a`, `Enter`, `b`. kk currently drops it: `Mode::
  resolve` returns `None` for any non-key input, so a paste is reported as
  `No action found`.
- `tuinix` reports a byte sequence it cannot decode as `Input::Unrecognized {
  bytes }`, which is exactly where an OSC 52 reply to an application's query
  would surface.

So the work is not new plumbing but recognising the input kk already receives
and deciding what it means.

## Guide-level explanation

The user copies text somewhere else, focuses kk, and presses `C-v`. The text
lands at the cursor as if typed. A multi-line paste is inserted as several
lines; it does not run one `Enter` per newline, so it cannot be mistaken for
commands.

This mirrors the existing behaviour of `C-y` (paste kk's clipboard) but reads
the terminal instead. Like `C-y`, it does not add to kk's own clipboard: after
pasting, `C-y` still inserts what `C-w`/`C-x w` last put there, because the two
clipboards stay separate.

There are two ways the text can reach kk, and the binding works with whichever
one the terminal offers:

- **Bracketed paste (preferred).** Many terminals map `C-Shift-V` (and
  sometimes `C-v`) to "paste from the system clipboard". When they do, kk
  receives `Input::Paste`. kk inserts it. This path needs no query and works
  even where OSC 52 reads are unsupported.
- **OSC 52 query (fallback).** When the terminal does not turn the key into a
  paste, kk sends `ESC ] 52 ; c ; ? BEL` and reads the reply. Not every
  terminal answers; where none does, the paste simply does not arrive and kk
  reports that the clipboard could not be read.

A user does not choose between these. Pressing `C-v` does the right thing on a
terminal that supports either.

## Reference-level explanation

### Binding

`C-v` is unbound in every mode today. Add it to the edit mode and the search
mode, resolved to a new `Action::ClipboardPasteFromTerminal` (name provisional;
see Unresolved questions):

- `src/binding.rs`, `resolve_edit`: `(true, KeyCode::Char('v')) =>
  act(Action::ClipboardPasteFromTerminal)`.
- `src/binding.rs`, `resolve_search`: the same chord, pasting into the query.

`C-y` keeps its meaning. The two bindings differ only in *source*: `C-y` is kk's
clipboard, `C-v` is the terminal's.

### Bracketed paste is no longer dropped

`Mode::resolve` currently ignores everything that is not a key. That is right
for mouse input (resolved separately in `app.rs`), but wrong for paste. Two
options, to be settled at implementation time:

- Handle `Input::Paste` in `App::handle_input` beside the mouse arm, before
  `mode.resolve`, inserting the bytes as text. This keeps the binding table
  about keys and lets a paste mean one thing in every mode.
- Give `resolve` a paste arm that returns the paste action, so its meaning can
  differ per mode (edit inserts; search inserts into the query).

The first is simpler and matches how mouse is already handled. The second is
more uniform if paste ever needs to mean different things per mode.

Either way the bytes are UTF-8 decoded, and the insertion reuses the existing
clipboard-insertion code path (`State::handle_clipboard_paste`) rather than a
second copy of the newline-splitting logic. The cleanest shape is for the edge
to place the decoded text into a temporary and call a method that inserts a
`&str`, shared by both `C-y` and `C-v`; the current `handle_clipboard_paste`
would be refactored so the buffer insertion is not tied to reading kk's own
clipboard.

### OSC 52 fallback

The fallback needs three things kk does not have yet:

1. **Writing the query.** `tuinix::TerminalDriver` (the version kk depends on)
   writes OSC 52 for `set_clipboard`; whether it exposes a way to write an
   arbitrary sequence, or a query specifically, must be confirmed against
   `0.7.2`. If not, the query is a `driver.write_all(b"\x1b]52;c;?\x07")`.
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

## Drawbacks

- More surface for little of the terminal ecosystem: most terminals answer
  neither `C-v`-as-paste nor OSC 52 reads, so on many setups the feature
  silently does nothing.
- OSC 52 reads are a security-sensitive channel. Asking for them in kk is fine,
  but users on terminals that answer must know a pasted string came from the
  system clipboard unchanged.
- `C-v` is many users' "insert verbatim" key (readline, emacs). Binding it to
  paste can surprise. See Unresolved questions.
- The refactor of `handle_clipboard_paste` to share the insertion path touches
  existing, working code for the benefit of the new command.

## Rationale and alternatives

- **Do nothing.** The user retypes or copies inside kk. Works today, but is the
  exact inconvenience the feature removes.
- **Bind `C-y` to read the terminal clipboard instead of kk's.** Rejected: kk's
  clipboard is the one `C-w` fills and is what a cut-then-paste expects; making
  `C-y` depend on the terminal would break a working flow.
- **Only bracketed paste, no `C-v`.** If the terminal already turns `C-v` into
  a paste, a binding is unnecessary. But not all terminals do, and users expect
  a key. Supporting the key as a trigger for the query covers more setups.
- **Only OSC 52, no bracketed paste.** Simpler in code but fails on the many
  terminals that ignore the query and on terminals that already send a paste
  (which would then be dropped and the query would double it).
- **Depend on a clipboard crate.** Pulls in system clipboard libraries, which
  assume an X11/Wayland session and undo kk's "minimum dependencies" stance.
  Out of scope; the terminal is the only clipboard kk should touch.

## Unresolved questions

- **The name.** `ClipboardPasteFromTerminal` is long. Alternatives: reuse
  `ClipboardPaste` for both with a source argument, or `PasteFromSystem`.
- **`C-v` vs another chord.** `C-v` collides with "insert verbatim" elsewhere.
  Is `C-v` the right key, or should it be `C-Shift-V` (if tuinix can report
  it) or a chord in the extension mode?
- **Which modes.** Should a paste be allowed while the legend-only or any
  non-editing state is active, or only in edit and search?
- **Does tuinix 0.7.2 expose writing an arbitrary sequence / an OSC 52 read?**
  This decides whether the fallback is a few lines or needs a tuinix change
  first. If it needs a tuinix change, this RFC is blocked on that.
- **Timeout behaviour.** How long, if at all, does kk wait for an OSC 52 reply
  before reporting failure? "Until the next input" is the current strawman but
  is not measured against a real terminal.

## Future possibilities

- If the terminal clipboard read works, `C-y` could be taught to fall back to
  it when kk's own clipboard is empty, making cut-then-paste and copy-then-paste
  behave the same way.
- A paste that arrives as `Input::Paste` and fails to decode as UTF-8 could be
  reported with the raw bytes, following the existing `Unrecognized` contract.
