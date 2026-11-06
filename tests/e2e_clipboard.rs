//! End-to-end clipboard import and export: `C-x y` (paste-clip) asks the
//! terminal for its clipboard, and `C-x w` (copy) hands it back.
//!
//! The unit tests check that a copy fills `State`'s in-process clipboard; these
//! check the other half -- the bytes that cross the terminal. A copy becomes an
//! OSC 52 write carrying exactly the copied text, a `C-x y` becomes an OSC 52
//! read, and the reply the host writes back becomes an insertion and nothing
//! else. Only a copy exports: a cut fills kk's own clipboard and asks the
//! terminal for nothing, and a copy that found no region exports nothing rather
//! than re-sending the previous contents.

mod e2e;

use e2e::{KkHarness, scratch_file};

#[test]
fn a_copy_reaches_the_terminal_as_an_osc_52_write() {
    let path = scratch_file("clipboard_copy.txt");
    std::fs::write(&path, "hello world\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);

    // Mark at the buffer start, move to the end of the line, and copy with
    // `C-x w`. The buffer is left alone, so this is a copy, not a cut.
    kk.send_ctrl(' ');
    kk.send_ctrl('e');
    kk.send_ctrl('x');
    kk.send_char('w');

    kk.wait_for_text("Copied");

    let requests = kk.child_requests();
    let exported: Vec<String> = requests
        .into_iter()
        .filter_map(|request| match request {
            termnix::ChildRequest::SetClipboard { text, append, .. } => {
                assert!(!append, "a copy replaces, it does not append");
                Some(String::from_utf8(text).expect("clipboard text is UTF-8"))
            }
            _ => None,
        })
        .collect();

    assert_eq!(exported, vec!["hello world".to_string()]);

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_cut_does_not_reach_the_terminal() {
    let path = scratch_file("clipboard_cut.txt");
    std::fs::write(&path, "hello world\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);

    // `C-w` cuts the marked region; it fills kk's own clipboard but must not
    // ask the terminal for anything.
    kk.send_ctrl(' ');
    kk.send_ctrl('e');
    kk.send_ctrl('w');
    kk.wait_for_text("Cut");

    let requests = kk.child_requests();
    assert!(
        !requests
            .iter()
            .any(|r| matches!(r, termnix::ChildRequest::SetClipboard { .. })),
        "a cut must not write to the terminal's clipboard: {requests:?}"
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_copy_without_a_mark_writes_nothing() {
    let path = scratch_file("clipboard_empty.txt");
    std::fs::write(&path, "hello world\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);

    // A copy that found no region reports so and records nothing, so it must
    // not hand the terminal anything -- in particular not the previous
    // contents, which an unconditional write would re-send.
    kk.send_ctrl('x');
    kk.send_char('w');
    kk.wait_for_text("No mark set");

    let requests = kk.child_requests();
    assert!(
        !requests
            .iter()
            .any(|r| matches!(r, termnix::ChildRequest::SetClipboard { .. })),
        "an empty copy must not write to the terminal's clipboard: {requests:?}"
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

/// An OSC 52 reply carrying `text`, as a host terminal answers a read.
///
/// The payload is base64 of `text`, framed the way OSC 52 frames any write
/// back: the system clipboard, `;`, the payload, and `ST`. `kk` decodes it
/// exactly as a terminal's reply would arrive on the PTY.
fn clipboard_reply(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    for chunk in text.as_bytes().chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for i in 0..4 {
            // Pad the last group with `=`, and never emit the digits that
            // stand for bytes the caller never passed.
            if i > chunk.len() {
                out.push(b'=');
            } else {
                out.push(alphabet[((n >> (18 - 6 * i)) & 0x3f) as usize]);
            }
        }
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

#[test]
fn ctrl_x_y_asks_the_terminal_for_its_clipboard() {
    let path = scratch_file("clipboard_query.txt");
    std::fs::write(&path, "hello\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.send_ctrl('x');
    kk.send_char('y');

    // The terminal reports the read as a request, the mirror image of the
    // write a copy produces.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut asked = false;
    while !asked {
        asked = kk.child_requests().iter().any(|request| {
            matches!(
                request,
                termnix::ChildRequest::GetClipboard {
                    selection: termnix::ClipboardSelection::Clipboard
                }
            )
        });
        assert!(!asked || std::time::Instant::now() < deadline);
        if !asked && std::time::Instant::now() >= deadline {
            panic!("C-x y never asked the terminal for its clipboard");
        }
    }

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn a_clipboard_reply_is_inserted_at_the_cursor() {
    let path = scratch_file("clipboard_reply.txt");
    std::fs::write(&path, "\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.send_ctrl('x');
    kk.send_char('y');

    // Answering as a terminal would: the reply is an OSC 52 write of the
    // clipboard's text, which the host sends to the child.
    kk.send_raw(&clipboard_reply("from the clipboard"));
    kk.wait_for_text("Pasted");
    kk.wait_for_text("from the clipboard");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn an_unanswered_clipboard_query_gives_up() {
    let path = scratch_file("clipboard_silent.txt");
    std::fs::write(&path, "hello\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);
    kk.send_ctrl('x');
    kk.send_char('y');

    // Most terminals do not answer an OSC 52 read. kk says so rather than
    // waiting for a reply that is not coming, and stays usable: the buffer is
    // reopened on the same key.
    kk.wait_for_text("Could not read the clipboard");
    kk.send_ctrl('e');
    kk.send_text("still here");
    kk.wait_for_text("still here");

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}

#[test]
fn an_osc_52_reply_nobody_asked_for_is_not_pasted() {
    let path = scratch_file("clipboard_unasked.txt");
    std::fs::write(&path, "hello\n").expect("write scratch file");

    let mut kk = KkHarness::open(&path);

    // No `C-x y` preceded this, so it is not the answer to a question kk
    // asked. A program or a terminal writing the clipboard must not be able to
    // type into the buffer; it keeps the ordinary unbound-input report.
    kk.send_raw(&clipboard_reply("injected"));
    kk.wait_for_text("UNRECOGNIZED");
    assert!(
        !kk.screen_contains("injected"),
        "an unasked-for reply reached the buffer:\n{}",
        kk.screen_text()
    );

    let status = kk.quit();
    assert!(status.success(), "kk exited with {status:?}");
}
