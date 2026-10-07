# RFC: A copy leaves to the terminal's clipboard

- Status: open

## Summary

`kk` pastes with `C-y` from an in-process clipboard it fills by cutting. It has
no copy: cutting (`C-k`, `C-w`) is the only way to fill that clipboard, and a
cut changes the buffer. This proposal adds `C-x w` to copy the marked region
without cutting it, and -- only for a copy -- also hands the text to the
terminal emulator as an OSC 52 sequence, so the copied text reaches the
operating system's clipboard (or, under tmux, the tmux buffer) and survives kk.

## Motivation

Today a cut is a dead end. A user who cuts a block out of a file to paste it
into the shell, or wants to copy a snippet to paste into a chat, has two
problems. The first is that copying means cutting: there is no way to put text
on the clipboard and leave the buffer as it was, so copying a line and then
pasting it elsewhere means cutting it out and putting it back. The second is
that even the cut text never leaves kk: it goes to a place only kk can read,
and it dies with the process. Select a paragraph with `C-w` to move it to a
chat window, and nothing arrives; quit kk, and the text is gone even for a
later kk session.

The mechanism to leave kk exists. Terminals have supported OSC 52 ("manipulate
selection data") for years: an application writes the base64 of the text to the
terminal, and the terminal puts it on the system clipboard. It works over SSH,
where a clipboard helper cannot, and tmux forwards it to its own buffer when
`set-clipboard` allows. The only missing piece is that kk never writes it -- and
that it has no copy to write.

## Guide-level explanation

A new command, `C-x w`, copies the marked region: the text goes to kk's own
clipboard (so `C-y` pastes it back, as after a cut) and is also sent to the
terminal, so pasting outside kk sees it. The buffer is left exactly as it was --
that is the whole point of copy over cut.

`C-x w` is the copy to `C-w`'s cut, in the same `C-x` extension map (`s` save,
`r` reload, `a`/`e` buffer start/end). The mark is used the same way it is for
`C-w`, and after the copy it is dropped and the cursor moves to the start of the
copied region, exactly as after a cut. So the sequence "mark, copy, paste"
leaves the cursor where "mark, cut, paste" would, minus the deletion.

What the user sees:

- Mark a block with `C-g`, `C-x w`, and it stays in the buffer; switch to the
  shell and paste it. The shell gets the block.
- Copy with `C-x w`, quit kk, open a browser, and paste. The text is still there,
  because it left kk when it was copied.
- Under tmux, the copy lands in the tmux buffer, so tmux's own paste sees it.

The feature is invisible on a good terminal and harmless on a terminal that
ignores it: if the terminal drops the sequence, the in-process clipboard still
holds the text and `C-y` still works. kk does not try to find out whether the
terminal accepted it -- see below on why it cannot.

Only a copy exports. A cut (`C-k`, `C-w`) still fills kk's clipboard and
nothing more: a cut is about removing text from the buffer, and quietly writing
the terminal's clipboard as a side effect of deleting would be a surprise. The
export is tied to the one command whose purpose is to put text on the
clipboard.

## Reference-level explanation

The core stays Sans I/O: it decides *that* a copy should reach the outside
clipboard and *what* the text is, and the edge does the writing. The core never
builds an escape sequence and never touches the terminal.

### The copy action

`Action::MarkCopy` sits beside `Action::MarkCut` in `src/action.rs`. The binding
is `(false, Char('w'))` in `resolve_ext` (the `C-x` map), and the `C-x` legend
gains a `w   copy` line. The handler, `State::handle_mark_copy`, shares its mark
bookkeeping with `handle_mark_cut` through a `take_mark_region` helper: it takes
the mark, resolves it to a `(start, end)` range and the region's text, and
reports a message the caller passes in (`No mark set` / `Nothing to cut` for a
cut, `No mark set` / `Nothing to copy` for a copy). The two handlers then diverge
by one call: the cut deletes the range, the copy does not.

A copy of an empty region (a mark with no text between it and the cursor)
reports `Nothing to copy` and records nothing. A successful copy reports
`Copied N characters`, where `N` is the character count of the region -- the
same count `C-w` reports for the text it cut.

### Core: a request, not a byte string

`kk::Clipboard` (`src/clipboard.rs`) holds one string, written by `write` and
`append`; a cut calls those and nothing leaves the process. `handle_mark_copy`
writes the region to the in-process clipboard the same way, and also records a
*pending export*: the text to send to the terminal.

The export is a plain value the edge can read, not a side effect, because the
core is Sans I/O:

```rust
/// Text that should be handed to the terminal's clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardExport {
    pub text: String,
}
```

`State` exposes it for the edge to take: `State::take_clipboard_export(&mut
self) -> Option<ClipboardExport>`, which returns the value and clears it so one
copy is sent once. There is no replace-or-append distinction: a copy always
replaces, because it is the only thing that exports and it always writes the
whole marked region. (kk's own clipboard has an `append`, used to accumulate a
run of cuts; nothing about a copy uses it, and the terminal only ever needs the
last, complete region.)

### Edge: hand it to tuinix

`src/app.rs` is the edge. After handling the action, it takes the pending export
and writes it:

```rust
if let Some(export) = self.state.take_clipboard_export() {
    // A write failure is the same kind of I/O error as drawing a frame, so it
    // propagates and ends kk. What is not reported is delivery -- see below.
    self.driver.set_clipboard(&export.text)?;
}
```

The natural place is `handle_action`, after the `match` that runs the handler.
Taking the export after every action is harmless because the take is `None` for
every action but `MarkCopy`.

The translation from text to an OSC 52 byte string belongs in `tuinix`, not in
kk. kk asks for "put this on the terminal's clipboard"; `tuinix` owns the
protocol, the same way it owns mouse reporting, resize handling, and the rest of
the terminal's control sequences. This keeps one place that knows terminal
escape codes, and it means a fix or a variation touches `tuinix` once for every
application built on it.

### What `tuinix` needs

`tuinix` already grew this method: `TerminalDriver::set_clipboard(&mut self,
text: &str) -> io::Result<()>` encodes `text` as base64 and writes
`ESC ] 52 ; c ; <base64> ST` to the terminal's output, then flushes. The
selection argument is `c` for the system clipboard; `ST` is `ESC \`. The write
is buffered like any other, and `set_clipboard` flushes so the sequence takes
effect without a following frame.

There is no `append_clipboard`. A copy replaces, so `set_clipboard` is the whole
surface kk needs. (Append would exist for a run of cuts, but a cut does not seed
the terminal's clipboard here.)

### What `termnix` needs

`termnix` already records the sequence. It hands an OSC 52 set to the caller as
`Event::RequestReceived(ChildRequest::SetClipboard { text, selection, append })`,
with `text` the decoded bytes, so a kk test can assert that the terminal
received the copied text and not just that some bytes were written. That is what
makes the end-to-end test the useful half. `termnix` is a dev-dependency (the
test terminal), so it is not part of kk's build or its running code; kk can
implement and ship the export whether or not `termnix` can decode it, and here
it can.

### Delivery is not reported, but a write failure is

Two things can go wrong, and kk treats them differently.

**Whether the terminal accepts the sequence cannot be observed.** By the time
kk writes it, the terminal either takes it or discards it, and *it cannot be
asked which*. OSC 52 has no reply; DA1 and similar queries do not reliably
report OSC 52 support, and a terminal that accepts the bytes may still have no
clipboard to write to (a headless server, a terminal with the feature off).
Under tmux, whether the sequence reaches the system clipboard depends on
`set-clipboard`, which is invisible to the application. So there is nothing to
report here: kk writes and moves on, with no message and no state change. This
is stronger than mouse reporting's report-and-swallow, because mouse reporting
at least has a failure to observe, while here there is only a guess to invent.
The in-process clipboard remains the source of truth for `C-y`, so a sequence a
terminal drops costs the user nothing inside kk.

**Whether the write itself succeeds can be observed, and is not swallowed.**
Writing the sequence is an ordinary write to the terminal's output, the same
kind of I/O as drawing a frame. If it fails -- the output is closed, the
write returns an error -- that is a real failure, not the terminal declining a
sequence it received, and it is not the sort of thing to hide behind `let _ =`.
kk already treats a failed file write as fatal (a save that cannot write ends
the action with an error), and a failed terminal write is the same shape: a
terminal kk cannot write to is one it cannot draw to either. So the write
propagates `io::Result` and ends kk, rather than being dropped. `set_clipboard`
writes and flushes, so the error it returns is what the write reported, not a
guess about delivery.

### Size

There is no size cap. The text is base64-encoded and written in one sequence, so
a very large copy makes a very large escape sequence, but what a terminal will
accept is the terminal's business and varies. Past whatever a terminal drops,
the in-process clipboard still holds the text and `C-y` still works, so a copy
that is too large to leave kk is still a copy inside kk. A cap would be a
number kk cannot ground in anything it can observe; leaving it out keeps the
behavior honest.

## Drawbacks

- A copy writes to the terminal, so it is not a purely in-process action. The
  write is one escape sequence, and a failure to write it ends kk; that is a new
  side effect, and a new way for a copy to fail.
- kk gains a command (`C-x w`) and a `ClipboardExport` type with a take method.
- Base64 encoding happens for a feature with no observable success, though the
  code lives in `tuinix` and is shared.

## Rationale and alternatives

- **Make a cut export, not a copy.** This was the first shape of the RFC: no new
  command, and `C-k`/`C-w` also write the terminal. It was rejected because it
  makes deleting text -- an action whose purpose is to remove -- also quietly
  overwrite the outside clipboard. A cut is about the buffer; a copy is about
  the clipboard. Tying the export to the copy is the honest pairing.
- **Rewrite cuts to be copies with the mark left in place.** Some editors make
  `C-w` a copy and require a separate kill for deletion. That changes what an
  existing chord does, and kk's `C-w` cutting is established. A new `C-x w`
  leaves `C-w` alone.
- **The core builds the OSC 52 bytes and returns them.** This would put the
  protocol in kk and let `tuinix` stay nearly untouched. It was rejected because
  it splits terminal knowledge across two crates: `tuinix` owns mouse reporting
  and every other sequence, and base64 plus OSC 52 in kk would be a second place
  that knows escape codes, and one that other `tuinix` users could not reuse.
  Keeping protocol in `tuinix` and intent in kk is the existing division.
- **Export only when a terminal is attached, and report when it is not.** There
  is no reliable way to tell, and kk always runs on a terminal. The check would
  not buy a real answer, so it is not made.
- **Keep kk's own clipboard and skip the terminal entirely.** That is today.
  kk's clipboard is right for kk's own paste (it must work with no terminal) but
  it is why a copy cannot leave kk; adding the export does not remove it.
- **Let tmux or the terminal read kk's clipboard.** There is no channel for
  that. OSC 52 is the channel applications use, and kk writing it is the only
  way in.
- **A configurable selection (primary, cut buffers).** The selection argument to
  OSC 52 can name other selections. That is more surface than the first version
  needs; leave it to a follow-up.
- **Detect support with a query and tell the user.** Not possible, as argued
  under "Delivery is not reported." A message saying "this terminal may not
  support OSC 52" is noise on the terminals that do.

## Impact

Ergonomics. A copy adds one chord and one clipboard path; a cut, `C-y`, the
buffer, and the file are unchanged. The costs are the `MarkCopy` action, a
binding and a legend line, `handle_mark_copy` (sharing the mark bookkeeping with
`handle_mark_cut`), the `ClipboardExport` type and take method in the core, and
one take-and-write in the edge. `tuinix::set_clipboard` and `termnix`'s OSC 52
capture already exist, so nothing new is needed from them.

## Future possibilities

- A cut that also exports, if the surprise of deleting-overwriting turns out to
  be what users want.
- Export other selections (primary), if the fixed `c` proves too narrow.
- If a terminal ever grows a way to acknowledge OSC 52, the silent write can
  become a reported one; until then, silence is the correct behavior.
