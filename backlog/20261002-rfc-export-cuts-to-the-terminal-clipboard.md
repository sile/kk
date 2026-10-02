# RFC: Cuts are also exported to the terminal's clipboard

- Status: open

## Summary

cut (`C-k`, `C-w`) fills kk's in-process clipboard, and that clipboard is the
only place the cut text goes. It lives and dies with the process, so the text
cannot be pasted into the shell, another editor, or a chat window, and it is
gone the moment kk exits. This proposal also hands each cut to the terminal
emulator as an OSC 52 sequence, so the cut text reaches the operating system's
clipboard (or, under tmux, the tmux buffer) and survives kk from the outside.

## Motivation

Today a cut is a dead end. A user who cuts a block out of a file to paste it
into the shell, or copies a snippet to paste into a chat, cannot: the text went
into a place only kk can read, and pressing `C-k` a second time overwrites it.
The in-process clipboard is right for kk's own `C-y` paste, which must work
without a terminal, but it is the whole story, and kk runs inside a terminal
that already has a way to reach the outside clipboard.

Concretely: select a paragraph with `C-w` to move it to a chat window, and
nothing arrives. Quit kk, and the cut text is gone even for a later kk session.
The only workaround is to leave kk, read the file from the shell, and copy from
there -- which is exactly the round trip the cut was meant to save.

The mechanism exists. Terminals have supported OSC 52 ("manipulate selection
data") for years: an application writes the base64 of the text to the terminal,
and the terminal puts it on the system clipboard. It works over SSH, where a
clipboard helper cannot, and tmux forwards it to its own buffer when
`set-clipboard` allows. The only missing piece is that kk never writes it.

## Guide-level explanation

Nothing changes about how a cut works in kk. `C-k` and `C-w` cut the same text
to the same in-process clipboard, and `C-y` pastes it back exactly as before.
The one addition is that the same text is now also sent to the terminal, so
pasting outside kk sees it:

- Cut a block in kk with `C-w`, switch to the shell, and paste it. The shell
gets the block.
- Cut with `C-k`, quit kk, open a browser, and paste. The text is still there,
  because it left kk when it was cut.
- Under tmux, the cut lands in the tmux buffer, so tmux's own paste sees it.

There is no new key to learn and no new mode. The feature is invisible on a
good terminal and harmless on a terminal that ignores it: if the terminal drops
the sequence, the in-process clipboard still holds the text and `C-y` still
works, exactly as today. kk does not try to find out whether the terminal
accepted it -- see below on why it cannot.

The scope is cuts only. kk has no copy command: it cuts, and pastes what was
cut. A copy that left the buffer alone would be a new command and is out of
scope here. `C-k` and `C-w` differ only in how much they cut; both export the
same way.

## Reference-level explanation

The core stays Sans I/O: it decides *that* a cut should reach the outside
clipboard and *what* the text is, and the edge does the writing. The core never
builds an escape sequence and never touches the terminal.

### Core: a request, not a byte string

`kk::Clipboard` (`src/clipboard.rs`) today holds one string and is written by
`write` and `append`. Cutting calls those two; nothing leaves the process. The
change is that the two methods also record a *pending export*: the text to send
and whether it replaced or was appended to what came before.

Because the core is Sans I/O, the pending export is a plain value the edge can
read, not a side effect. A shape that fits the existing code:

```rust
/// Text that should be handed to the terminal's clipboard, and how it
/// combines with whatever is already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardExport {
    pub text: String,
    pub kind: ClipboardExportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardExportKind {
    /// Replaces what the terminal holds.
    Replace,
    /// Appends to what the terminal holds.
    Append,
}
```

`Clipboard::write` records `Replace`, `Clipboard::append` records `Append`, and
both store the text they were given. `State` exposes the pending export for the
edge to take, e.g. `State::take_clipboard_export(&mut self) ->
Option<ClipboardExport>`, which returns the value and clears it so one cut is
sent once.

`Replace` versus `Append` mirrors `write` versus `append`, which the editor
already uses: a run of cuts it treats as one collects into a single clipboard
entry, and the terminal should see the same thing -- the first cut replaces,
the rest append. Sending only the newest slice each time would leave the
terminal holding the last piece instead of the whole run.

A cut with an empty result (for example `C-k` on an empty line) writes or
appends the empty string. That is a no-op for the in-process clipboard and must
be a no-op for the export too, so an empty text is not recorded as pending.

### Edge: hand it to tuinix

`src/app.rs` is the edge. After handling an action that can cut, it takes the
pending export and writes it:

```rust
if let Some(export) = self.state.take_clipboard_export() {
    // Mouse reporting's precedent: a convenience that a terminal may refuse.
    // The failure is swallowed, not reported.
    let _ = match export.kind {
        kk::ClipboardExportKind::Replace => self.driver.set_clipboard(&export.text),
        kk::ClipboardExportKind::Append => self.driver.append_clipboard(&export.text),
    };
}
```

The natural place is `handle_action`, where the cut actions already are
(`LineCutTail`, `MarkCut`, and the search prompt's cut-query if it fills the
clipboard). The concern is only the actions that reach `Clipboard::write` or
`append`; taking the export after every action is harmless because the take is
`None` when nothing cut.

The translation from text to an OSC 52 byte string belongs in `tuinix`, not in
kk. kk asks for "put this on the terminal's clipboard"; `tuinix` owns the
protocol, the same way it owns mouse reporting, resize handling, and the rest
of the terminal's control sequences. This keeps one place that knows terminal
escape codes, and it means a fix or a variation (see below) touches `tuinix`
once for every application built on it.

### What `tuinix` needs

`tuinix`'s `TerminalDriver` writes raw control sequences already -- for example
`enable_mouse_reporting`, which writes `\x1b[?1000h` and the like to its output.
OSC 52 is the same kind of write, so the addition is a method beside that one:

```rust
impl TerminalDriver {
    /// Replaces the terminal's clipboard with `text` (OSC 52).
    pub fn set_clipboard(&mut self, text: &str) -> std::io::Result<()>
    where
        Self: Write;
    /// Appends `text` to the terminal's clipboard (OSC 52, "append").
    pub fn append_clipboard(&mut self, text: &str) -> std::io::Result<()>;
}
```

Each encodes `text` as base64 and writes `ESC ] 52 ; c ; <base64> ST`. The
selection argument is `c` for the system clipboard. `ST` is `ESC \`; a bare
`BEL` is also accepted by some terminals, and `tuinix` should pick one and say
so.

Base64 is the one real piece of work. `tuinix` has no base64 dependency today
(neither does kk, and adding one to either for this is a cost to weigh); the
encoding is short enough to write out, and the RFC leaves that choice to the
`tuinix` change. The RFC does not require a new crate.

### What `termnix` needs

`termnix` is the test terminal. It handles OSC 0 and 2 (the window title) and
ignores every other OSC, by design -- there is no clipboard to keep. For a test
to see the sequence at all, `termnix` needs to keep it: an OSC 52 arm in its
dispatch that records the decoded text (and whether it was a set or an append)
in a field the test can read, the way the title is exposed today. Without that,
a test can assert only that the bytes were written, not that they encoded the
right text, which is the useful half.

This is a change to `termnix`, not to kk, and it is what makes the kk test
meaningful. If `termnix` keeps the sequence, a kk test can cut and assert the
terminal received the cut text; if it does not, the test can only watch the raw
bytes.

### Landing order

kk's own implementation depends on `tuinix` alone. `termnix` is a
dev-dependency (the test terminal), so it is not part of kk's build or its
running code: kk can implement and ship the export whether or not `termnix`
ever learns OSC 52.

The two crates are therefore not the same kind of prerequisite:

- **`tuinix` is required.** kk cannot write to the terminal except through
  `TerminalDriver`, so until `set_clipboard`/`append_clipboard` exist there,
  the kk change has nothing to call and cannot land.
- **`termnix` is for the test, not for kk.** It is what lets a kk test assert
  that the cut text was encoded and delivered correctly -- that the base64
  decoded back to the text that was cut, and that a set was told apart from an
  append. Without it, kk still lands; the test can only watch the raw bytes
  `tuinix` wrote (in `tuinix`'s own unit test, or by capturing the output
  stream in kk's). That is a weaker assertion -- the bytes were written, not
  that they were right -- but it does not block the feature.

So the intended order is `tuinix` first, then kk, with the `termnix` arm
landing whenever a kk test wants the stronger assertion. The RFC is written as
if `tuinix`'s methods exist and lands when they do. Filing issues against
`tuinix` and `termnix` is a separate step from applying this change, and this
RFC does not depend on that step having happened when it is written -- it
depends on `tuinix`'s methods happening before kk lands.

### Failure is not reported

Mouse reporting sets a message when it is unavailable. OSC 52 is weaker still:
by the time kk writes it, there is no failure to report. The terminal either
accepts the sequence or discards it, and *it cannot be asked which*. OSC 52 has
no reply; DA1 and similar queries do not reliably report OSC 52 support, and a
terminal that accepts the bytes may still have no clipboard to write to
(a headless server, a terminal with the feature off). Under tmux, whether the
sequence reaches the system clipboard depends on `set-clipboard`, which is
invisible to the application.

So kk does not try: it writes and moves on. No message, no error, no state
change. This is stronger than mouse reporting's report-and-swallow, because
mouse reporting at least has a failure to observe. Here there is nothing to
observe, and inventing a message would be guessing. The in-process clipboard
remains the source of truth for `C-y`, so a dropped sequence costs the user
nothing inside kk.

### Size cap

The text is base64-encoded and written in one sequence. A very large cut would
produce a very large escape sequence, and terminals vary in how much they
accept -- some cap the sequence, some cap the clipboard, some drop a sequence
past a length. The RFC caps the export at **10 MB** (`10 * 1024 * 1024` bytes
of text; base64 makes the sequence about a third larger). Past the cap the text
is not exported, but it is still cut to the in-process clipboard, so `C-y`
keeps working and only the outside copy is skipped. The cap is a hard-coded
constant for now; making it configurable is a later question, not this one.

The cap is checked before encoding, in kk or in `tuinix` -- the RFC prefers
tuinix, beside the encoding, so every caller gets the same bound. It is a
silent skip, like every other failure here.

## Drawbacks

- A cut now writes to the terminal, so a cut is no longer a purely in-process
action. The write is one escape sequence and is ignored on failure, but it is a
new side effect in a path that used to be self-contained.
- kk depends on `tuinix` gaining two methods and `termnix` gaining a field,
  and the kk change cannot land until they do. A proposal blocked on another
  crate is slower to land than one that is not.
- Base64 encoding is new code (wherever it lives) plus a cap to reason about,
  for a feature with no observable success.
- The core grows a `ClipboardExport` type and a take method for something only
  the edge uses.

## Rationale and alternatives

- **The core builds the OSC 52 bytes and returns them.** This would put the
  protocol in kk and let `tuinix` stay nearly untouched. It was rejected because
  it splits terminal knowledge across two crates: `tuinix` owns mouse reporting
  and every other sequence, and base64 plus OSC 52 in kk would be a second
  place that knows escape codes, and one that other `tuinix` users could not
  reuse. Keeping protocol in `tuinix` and intent in kk is the existing division.
- **Export only when a terminal is attached, and report when it is not.** There
  is no reliable way to tell, and `kk` always runs on a terminal. The check
  would not buy a real answer, so it is not made.
- **Keep kk's own clipboard and skip the terminal entirely.** That is today.
  kk's clipboard is right for kk's own paste (it must work with no terminal) but
  it is why a cut cannot leave kk; adding the export does not remove it.
- **A `copy` command that leaves the buffer alone.** kk has no copy today, and
  adding one changes the editing model rather than where a cut goes. It can be
  its own proposal if wanted; this RFC is about the cut kk already has.
- **Let tmux or the terminal read kk's clipboard.** There is no channel for
  that. OSC 52 is the channel applications use, and kk writing it is the only
  way in.
- **A configurable size cap and selection (primary, cut buffers).** The
  selection argument to OSC 52 can name other selections, and the cap could be a
  setting. Both are more surface than the first version needs; leave them to a
  follow-up.
- **Detect support with a query and tell the user.** Not possible, as argued
  under "Failure is not reported." A message saying "this terminal may not
  support OSC 52" is noise on the terminals that do.

## Impact

Ergonomics. A cut still cuts; `C-y` still pastes; the buffer and the file are
untouched. The change is what a cut leaves behind outside the process. The
costs are the `ClipboardExport` type and take method in the core, one take-and-
write in the edge, two methods in `tuinix`, an OSC 52 arm in `termnix`, and a
size cap. There is no new key binding and no change to any existing action.

## Future possibilities

- A `copy` command that exports without cutting, sharing this export path.
- A configurable cap, and other selections (primary), if the fixed `c` proves
  too narrow.
- The same export for `C-w` while a search prompt is open, if that prompt fills
  the clipboard.
- If a terminal ever grows a way to acknowledge OSC 52, the silent write can
  become a reported one; until then, silence is the correct behavior.
