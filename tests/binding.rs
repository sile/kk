//! Properties and examples of the hard-coded input bindings.

/// A plain character key with no modifiers.
fn char_key(ch: char) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: false,
        alt: false,
        code: tuinix::KeyCode::Char(ch),
    }
}

/// A ctrl chord on a character key.
fn ctrl_key(ch: char) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: true,
        alt: false,
        code: tuinix::KeyCode::Char(ch),
    }
}

/// An alt chord on a character key.
fn alt_key(ch: char) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: false,
        alt: true,
        code: tuinix::KeyCode::Char(ch),
    }
}

/// A ctrl+alt chord on a character key.
fn alt_ctrl_key(ch: char) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: true,
        alt: true,
        code: tuinix::KeyCode::Char(ch),
    }
}

/// A bare special key with no modifiers.
fn code_key(code: tuinix::KeyCode) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: false,
        alt: false,
        code,
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

/// Every context the resolver knows about.
const CONTEXTS: [kk::Context; 3] = [kk::Context::Edit, kk::Context::Search, kk::Context::Ext];

/// The key chords the built-in tables are expected to bind, gathered from the
/// same vocabulary the tests below spell out.
fn built_in_keys() -> Vec<tuinix::KeyInput> {
    let mut keys = vec![
        ctrl_key('c'),
        ctrl_key('g'),
        ctrl_key('r'),
        ctrl_key('s'),
        ctrl_key('x'),
        ctrl_key('y'),
        ctrl_key('w'),
        ctrl_key('l'),
        ctrl_key('k'),
        ctrl_key('a'),
        ctrl_key('e'),
        ctrl_key('d'),
        ctrl_key('h'),
        ctrl_key('j'),
        ctrl_key('p'),
        ctrl_key('n'),
        ctrl_key('b'),
        ctrl_key('f'),
        ctrl_key('u'),
        ctrl_key(' '),
        ctrl_key('`'),
        // The Ext context drops the ctrl prefix, so its own chords are the bare
        // letters; case is what separates save from force-save.
        char_key('s'),
        char_key('S'),
        char_key('r'),
        char_key('a'),
        char_key('e'),
        code_key(tuinix::KeyCode::Up),
        code_key(tuinix::KeyCode::Down),
        code_key(tuinix::KeyCode::Left),
        code_key(tuinix::KeyCode::Right),
        code_key(tuinix::KeyCode::Enter),
        code_key(tuinix::KeyCode::Backspace),
        code_key(tuinix::KeyCode::Delete),
        code_key(tuinix::KeyCode::Tab),
        code_key(tuinix::KeyCode::BackTab),
        code_key(tuinix::KeyCode::Escape),
    ];
    keys.sort_by_key(|key| format!("{key:?}"));
    keys
}

/// Resolves `key` in `context` and returns the action it ran.
fn action_of(context: kk::Context, key: tuinix::KeyInput) -> Option<kk::Action> {
    kk::resolve(context, &tuinix::Input::Key(key)).and_then(|resolved| resolved.action)
}

#[test]
fn every_built_in_chord_resolves_somewhere() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let keys = built_in_keys();
    assert!(!keys.is_empty(), "the chord vocabulary is not empty");

    let mut runner = noprop::Runner::new(seed);
    runner.run(keys.len(), |ctx| {
        let key = noprop::sample_choice(ctx, &keys);
        let input = tuinix::Input::Key(key);

        let resolved = CONTEXTS.iter().any(|&c| kk::resolve(c, &input).is_some());
        assert!(resolved, "{key:?} resolves in no context at all");
        Ok(())
    })?;

    Ok(())
}

#[test]
fn a_resolved_binding_does_something_or_switches_context() {
    for &context in &CONTEXTS {
        for key in built_in_keys() {
            if let Some(resolved) = kk::resolve(context, &tuinix::Input::Key(key)) {
                assert!(
                    resolved.action.is_some() || resolved.context.is_some(),
                    "{key:?} in {context:?} neither acts nor switches context"
                );
            }
        }
    }
}

#[test]
fn printable_characters_are_text_in_edit() {
    for ch in ['a', 'Z', '0', ' ', '~', '\u{3042}', '\u{1f600}'] {
        assert!(
            matches!(
                action_of(kk::Context::Edit, char_key(ch)),
                Some(kk::Action::CharInsert)
            ),
            "{ch:?} is printable and should insert text"
        );
    }
}

#[test]
fn edit_rejects_control_characters() {
    // A control character is not text; it is either a binding or nothing.
    for ch in ['\n', '\t', '\r', '\u{7f}', '\u{0}'] {
        assert!(
            !matches!(
                action_of(kk::Context::Edit, char_key(ch)),
                Some(kk::Action::CharInsert)
            ),
            "{ch:?} is a control character and must not insert text"
        );
    }

    // A ctrl chord is a binding, not text.
    assert!(!matches!(
        action_of(kk::Context::Edit, ctrl_key('z')),
        Some(kk::Action::CharInsert)
    ));

    // A special key is never text either.
    assert!(!matches!(
        action_of(kk::Context::Edit, code_key(tuinix::KeyCode::Enter)),
        Some(kk::Action::CharInsert)
    ));
}

#[test]
fn escape_toggles_the_legend_in_every_context() {
    // A lone `ESC` is committed by the decoder as `Escape` with no modifiers,
    // so the plain code is the whole chord in every context.
    for &context in &CONTEXTS {
        assert!(
            matches!(
                action_of(context, code_key(tuinix::KeyCode::Escape)),
                Some(kk::Action::LegendToggle)
            ),
            "{context:?} does not toggle the legend on Escape"
        );
    }
}

#[test]
fn no_ctrl_chord_toggles_the_legend() {
    // `C-?` used to be the toggle, but the decoder turns it into a DEL byte's
    // neighbour and it read as a backspace; Escape is unambiguous.
    for &context in &CONTEXTS {
        for ch in ['\u{7f}', '?', '/', 'u'] {
            assert!(
                !matches!(
                    action_of(context, ctrl_key(ch)),
                    Some(kk::Action::LegendToggle)
                ),
                "{context:?} still toggles the legend on C-{ch:?}"
            );
        }
    }
}

#[test]
fn undo_is_bound_to_ctrl_u_alone() {
    assert!(
        matches!(
            action_of(kk::Context::Edit, ctrl_key('u')),
            Some(kk::Action::BufferUndo)
        ),
        "C-u no longer undoes"
    );
    assert!(
        !matches!(
            action_of(kk::Context::Edit, ctrl_key('/')),
            Some(kk::Action::BufferUndo)
        ),
        "C-/ must not undo any more"
    );
    assert!(
        !matches!(
            action_of(kk::Context::Edit, code_key(tuinix::KeyCode::Escape)),
            Some(kk::Action::BufferUndo)
        ),
        "Escape must not undo; it toggles the legend"
    );
}

#[test]
fn ctrl_c_quits_the_edit_context() {
    assert!(matches!(
        action_of(kk::Context::Edit, ctrl_key('c')),
        Some(kk::Action::Quit)
    ));
}

#[test]
fn every_other_context_has_a_way_back_to_edit() {
    // Search and Ext are entered from Edit, so a chord that leaves for Edit is
    // what keeps them from trapping the editor. Search names the two ways out
    // after where the cursor ends up, so both count.
    for &context in &[kk::Context::Search, kk::Context::Ext] {
        let returns = built_in_keys().into_iter().any(|key| {
            matches!(
                kk::resolve(context, &tuinix::Input::Key(key)),
                Some(kk::Resolved {
                    action: Some(
                        kk::Action::Cancel | kk::Action::SearchCancel | kk::Action::SearchAccept
                    ),
                    context: Some(kk::Context::Edit),
                })
            )
        });
        assert!(returns, "{context:?} cannot return to the edit context");
    }
}

#[test]
fn the_ext_context_binds_its_chords_after_ctrl_x() {
    // The buffer-level commands live behind `C-x`, which is the way into the
    // Ext context. Inside Ext the ctrl prefix is dropped, so each is a plain
    // letter that runs and returns to Edit.
    for (ch, expected) in [('r', 0), ('a', 1), ('e', 2)] {
        let resolved = kk::resolve(kk::Context::Ext, &tuinix::Input::Key(char_key(ch)))
            .unwrap_or_else(|| panic!("{ch} is not bound in Ext"));
        assert_eq!(
            resolved.context,
            Some(kk::Context::Edit),
            "{ch} does not return to Edit"
        );
        assert!(
            matches!(
                (expected, &resolved.action),
                (0, Some(kk::Action::BufferReload))
                    | (1, Some(kk::Action::CursorBufferStart))
                    | (2, Some(kk::Action::CursorBufferEnd))
            ),
            "{ch} carries out the wrong action: {:?}",
            resolved.action
        );
    }
}

#[test]
fn the_ext_context_binds_force_save_to_an_upper_case_s() {
    let resolved = kk::resolve(kk::Context::Ext, &tuinix::Input::Key(char_key('S')))
        .expect("S is bound in Ext");
    assert_eq!(resolved.context, Some(kk::Context::Edit));
    assert!(
        matches!(resolved.action, Some(kk::Action::BufferForceSave)),
        "S force-saves: {:?}",
        resolved.action
    );

    // The lower-case letter must still be the checking save.
    let plain = kk::resolve(kk::Context::Ext, &tuinix::Input::Key(char_key('s')))
        .expect("s is bound in Ext");
    assert!(matches!(plain.action, Some(kk::Action::BufferSave)));
}

#[test]
fn alt_is_ignored_in_every_context() {
    // Alt is not a modifier `kk` acts on, so an alt chord resolves exactly as
    // the same chord without alt: `M-r` is plain `r`, and `M-C-r` is `C-r`.
    for &context in &CONTEXTS {
        for ch in ['r', '<', '>', 'm', 'l', 'w', 'g', 'z', 'c'] {
            assert_eq!(
                format!("{:?}", action_of(context, alt_key(ch))),
                format!("{:?}", action_of(context, char_key(ch))),
                "M-{ch} does not resolve as {ch} in {context:?}"
            );
            assert_eq!(
                format!("{:?}", action_of(context, alt_ctrl_key(ch))),
                format!("{:?}", action_of(context, ctrl_key(ch))),
                "M-C-{ch} does not resolve as C-{ch} in {context:?}"
            );
        }
    }
}

#[test]
fn edit_accepts_an_alt_chord_as_text() {
    // The flip side of ignoring alt: `M-z` is `z`, which inserts.
    assert!(matches!(
        action_of(kk::Context::Edit, alt_key('z')),
        Some(kk::Action::CharInsert)
    ));
}

#[test]
fn each_context_resolves_its_own_chords() {
    // Both contexts bind `C-k`, but each keeps its own clipboard: the two
    // actions differ, so the kill never lands in the other's clipboard.
    assert!(matches!(
        action_of(kk::Context::Edit, ctrl_key('k')),
        Some(kk::Action::LineDelete)
    ));
    assert!(matches!(
        action_of(kk::Context::Search, ctrl_key('k')),
        Some(kk::Action::SearchKillQuery)
    ));

    // Search has its own keys, which Edit does not.
    assert!(action_of(kk::Context::Search, code_key(tuinix::KeyCode::Tab)).is_some());
    assert!(action_of(kk::Context::Edit, code_key(tuinix::KeyCode::Tab)).is_none());
}

#[test]
fn non_key_input_never_resolves() {
    for &context in &CONTEXTS {
        assert!(kk::resolve(context, &tuinix::Input::Mouse(left_press())).is_none());
        assert!(
            kk::resolve(
                context,
                &tuinix::Input::Unrecognized {
                    bytes: b"\x1b[?".to_vec()
                }
            )
            .is_none()
        );
        assert!(
            kk::resolve(
                context,
                &tuinix::Input::Paste {
                    bytes: b"hi".to_vec()
                }
            )
            .is_none()
        );
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
