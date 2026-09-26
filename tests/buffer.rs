//! Properties and examples of the buffer's text round trip.

/// Text ending in a newline, in the exact shape [`kk::TextBuffer::to_text`]
/// produces and a file normally holds: every line, including a blank one in
/// the middle, is newline-terminated.
fn sample_text(ctx: &mut noprop::TestCaseContext) -> String {
    let lines = noprop::sample_usize_in(ctx, 1..=4);
    let mut text = String::new();
    for _ in 0..lines {
        let len = noprop::sample_usize_in(ctx, 0..=8);
        text.push_str(&noprop::sample_ascii_printable_string(ctx, len));
        text.push('\n');
    }
    text
}

#[test]
fn from_text_then_to_text_round_trips_newline_terminated_text() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let text = sample_text(ctx);
        let buffer = kk::TextBuffer::from_text(&text);

        assert_eq!(buffer.to_text(), text, "round trip changed {text:?}");
        assert_eq!(
            buffer.rows(),
            text.matches('\n').count(),
            "row count for {text:?}"
        );
        Ok(())
    })?;

    Ok(())
}

#[test]
fn from_text_of_unterminated_text_gains_a_final_newline() {
    let buffer = kk::TextBuffer::from_text("one\ntwo");

    assert_eq!(buffer.to_text(), "one\ntwo\n");
    assert_eq!(buffer.rows(), 2);
}

#[test]
fn an_empty_file_has_no_lines_and_saves_as_a_single_newline() {
    // The one shape `from_text` and `to_text` do not round-trip: a file with
    // no lines at all. Opening an empty file and saving it adds a newline.
    let buffer = kk::TextBuffer::from_text("");

    assert_eq!(buffer.rows(), 0);
    assert_eq!(buffer.to_text(), "\n");
}

#[test]
fn a_blank_line_in_the_middle_survives_the_round_trip() {
    let buffer = kk::TextBuffer::from_text("a\n\nb\n");

    assert_eq!(buffer.rows(), 3);
    assert_eq!(buffer.to_text(), "a\n\nb\n");
}
