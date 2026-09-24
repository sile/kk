//! Properties and examples of the hard-coded input bindings.

/// A plain character key with no modifiers.
fn char_key(ch: char) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: false,
        alt: false,
        code: tuinix::KeyCode::Char(ch),
    }
}

/// A left press at the origin, with no modifier keys held.
fn left_press() -> tuinix::MouseInput {
    tuinix::MouseInput {
        kind: tuinix::MouseInputKind::LeftPress,
        position: tuinix::Position::ORIGIN,
        ctrl: false,
        alt: false,
        shift: false,
    }
}

/// Every trigger spec the built-in tables use, so the round trip covers the
/// spelling the code actually relies on.
fn built_in_specs() -> Vec<String> {
    let mut specs = Vec::new();
    for context in [
        kk::Context::Main,
        kk::Context::Grep,
        kk::Context::Ext,
        kk::Context::Goto,
    ] {
        for binding in kk::Bindings::new().get(context) {
            for trigger in &binding.triggers {
                specs.push(trigger.to_string());
            }
        }
    }
    specs.sort();
    specs.dedup();
    specs
}

#[test]
fn every_built_in_trigger_round_trips_through_its_text_form() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let specs = built_in_specs();
    assert!(!specs.is_empty(), "the binding table is not empty");

    let mut runner = noprop::Runner::new(seed);
    runner.run(specs.len(), |ctx| {
        let spec = noprop::sample_choice(ctx, &specs);
        let matcher: kk::InputMatcher = spec.parse().expect("a built-in spec parses");

        assert_eq!(matcher.to_string(), spec, "round trip changed {spec:?}");
        Ok(())
    })?;

    Ok(())
}

#[test]
fn printable_matches_ordinary_characters_only() {
    let printable = kk::InputMatcher::Printable;

    for ch in ['a', 'Z', '0', ' ', '~', '\u{3042}', '\u{1f600}'] {
        assert!(
            printable.matches(&tuinix::Input::Key(char_key(ch))),
            "{ch:?} is printable input"
        );
    }
}

#[test]
fn printable_rejects_control_and_modified_keys() {
    let printable = kk::InputMatcher::Printable;

    for ch in ['\n', '\t', '\r', '\u{7f}', '\u{0}'] {
        assert!(
            !printable.matches(&tuinix::Input::Key(char_key(ch))),
            "{ch:?} is a control character"
        );
    }

    let ctrl = tuinix::KeyInput {
        ctrl: true,
        alt: false,
        code: tuinix::KeyCode::Char('x'),
    };
    let alt = tuinix::KeyInput { alt: true, ..ctrl };
    assert!(!printable.matches(&tuinix::Input::Key(ctrl)));
    assert!(!printable.matches(&tuinix::Input::Key(alt)));
    assert!(
        !printable.matches(&tuinix::Input::Key(tuinix::KeyInput {
            ctrl: false,
            alt: false,
            code: tuinix::KeyCode::Enter,
        })),
        "Enter is not a printable character"
    );
}

#[test]
fn a_key_matcher_matches_only_its_exact_chord() {
    let matcher = kk::InputMatcher::Key(tuinix::KeyInput {
        ctrl: true,
        alt: false,
        code: tuinix::KeyCode::Char('a'),
    });

    assert!(matcher.matches(&tuinix::Input::Key(tuinix::KeyInput {
        ctrl: true,
        alt: false,
        code: tuinix::KeyCode::Char('a'),
    })));
    assert!(
        !matcher.matches(&tuinix::Input::Key(char_key('a'))),
        "the modifier is part of the chord"
    );
    assert!(!matcher.matches(&tuinix::Input::Key(tuinix::KeyInput {
        ctrl: false,
        alt: true,
        code: tuinix::KeyCode::Char('a'),
    })));
}

#[test]
fn a_key_matcher_never_matches_mouse_input() {
    let matcher = kk::InputMatcher::Key(char_key('a'));

    assert!(!matcher.matches(&tuinix::Input::Mouse(left_press())));
}

#[test]
fn each_context_resolves_to_its_own_table() {
    let bindings = kk::Bindings::new();

    // Main has the editing chords; Goto does not.
    let triggers = |context| {
        bindings
            .get(context)
            .iter()
            .flat_map(|binding| binding.triggers.iter().map(|t| t.to_string()))
            .collect::<Vec<_>>()
    };

    assert!(triggers(kk::Context::Main).contains(&"C-k".to_string()));
    assert!(!triggers(kk::Context::Goto).contains(&"C-k".to_string()));
    assert!(
        triggers(kk::Context::Goto).contains(&"p".to_string()),
        "goto has its own keys"
    );
}

#[test]
fn the_main_context_can_leave_the_editor() {
    let bindings = kk::Bindings::new();

    let has_quit = bindings
        .get(kk::Context::Main)
        .iter()
        .any(|binding| matches!(binding.action, Some(kk::Action::Quit)));

    assert!(has_quit, "nothing in the main context quits");
}

#[test]
fn every_non_main_context_has_a_way_back_to_main() {
    let bindings = kk::Bindings::new();

    // Grep, Ext, and Goto are entered from Main, so a chord that returns to
    // Main is what keeps them from trapping the editor.
    for context in [kk::Context::Grep, kk::Context::Ext, kk::Context::Goto] {
        let returns = bindings.get(context).iter().any(|binding| {
            binding.context == Some(kk::Context::Main)
                && matches!(binding.action, Some(kk::Action::Cancel))
        });
        assert!(returns, "{context:?} cannot return to the main context");
    }
}

#[test]
fn every_binding_does_something_or_switches_context() {
    for context in [
        kk::Context::Main,
        kk::Context::Grep,
        kk::Context::Ext,
        kk::Context::Goto,
    ] {
        for binding in kk::Bindings::new().get(context) {
            assert!(
                binding.action.is_some() || binding.context.is_some(),
                "a binding in {context:?} neither acts nor switches context"
            );
        }
    }
}

#[test]
fn input_display_covers_unrecognized_and_paste() {
    let unrecognized = kk::input(&tuinix::Input::Unrecognized {
        bytes: b"\x1b[?".to_vec(),
    });
    assert_eq!(unrecognized, "<UNRECOGNIZED>");

    let paste = kk::input(&tuinix::Input::Paste {
        bytes: b"hi".to_vec(),
    });
    assert_eq!(paste, "<PASTE>");
}

#[test]
fn input_display_renders_keys_and_mouse() {
    assert_eq!(kk::input(&tuinix::Input::Key(char_key('a'))), "a");
    assert_eq!(
        kk::input(&tuinix::Input::Key(tuinix::KeyInput {
            ctrl: true,
            alt: false,
            code: tuinix::KeyCode::Char('a'),
        })),
        "C-a"
    );
    assert_eq!(
        kk::input(&tuinix::Input::Mouse(left_press())),
        "<LEFTCLICK>"
    );
}
