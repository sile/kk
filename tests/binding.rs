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

/// An alt chord on a special key.
fn alt_code_key(code: tuinix::KeyCode) -> tuinix::KeyInput {
    tuinix::KeyInput {
        ctrl: false,
        alt: true,
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

/// Every mode the resolver knows about.
const MODES: [kk::Mode; 3] = [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext];

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
        ctrl_key('p'),
        ctrl_key('n'),
        ctrl_key('b'),
        ctrl_key('f'),
        ctrl_key('u'),
        ctrl_key(' '),
        ctrl_key('`'),
        // The Ext mode drops the ctrl prefix, so its own chords are the bare
        // letters. `s` is the deliberate one there and force-saves.
        char_key('s'),
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

/// Resolves `key` in `mode` and returns the action it ran.
fn action_of(mode: kk::Mode, key: tuinix::KeyInput) -> Option<kk::Action> {
    mode.resolve(&tuinix::Input::Key(key))
        .and_then(|resolved| resolved.action)
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

        let resolved = MODES.iter().any(|&c| c.resolve(&input).is_some());
        assert!(resolved, "{key:?} resolves in no mode at all");
        Ok(())
    })?;

    Ok(())
}

#[test]
fn a_resolved_binding_does_something_or_switches_mode() {
    for &mode in &MODES {
        for key in built_in_keys() {
            if let Some(resolved) = mode.resolve(&tuinix::Input::Key(key)) {
                assert!(
                    resolved.action.is_some() || resolved.mode.is_some(),
                    "{key:?} in {mode:?} neither acts nor switches mode"
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
                action_of(kk::Mode::Edit, char_key(ch)),
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
                action_of(kk::Mode::Edit, char_key(ch)),
                Some(kk::Action::CharInsert)
            ),
            "{ch:?} is a control character and must not insert text"
        );
    }

    // A ctrl chord is a binding, not text.
    assert!(!matches!(
        action_of(kk::Mode::Edit, ctrl_key('z')),
        Some(kk::Action::CharInsert)
    ));

    // A special key is never text either.
    assert!(!matches!(
        action_of(kk::Mode::Edit, code_key(tuinix::KeyCode::Enter)),
        Some(kk::Action::CharInsert)
    ));
}

#[test]
fn escape_toggles_the_legend_in_every_mode() {
    // A lone `ESC` is committed by the decoder as `Escape` with no modifiers,
    // so the plain code is the whole chord in every mode.
    for &mode in &MODES {
        assert!(
            matches!(
                action_of(mode, code_key(tuinix::KeyCode::Escape)),
                Some(kk::Action::LegendToggle)
            ),
            "{mode:?} does not toggle the legend on Escape"
        );
    }
}

#[test]
fn no_ctrl_chord_toggles_the_legend() {
    // `C-?` used to be the toggle, but the decoder turns it into a DEL byte's
    // neighbour and it read as a backspace; Escape is unambiguous.
    for &mode in &MODES {
        for ch in ['\u{7f}', '?', '/', 'u'] {
            assert!(
                !matches!(
                    action_of(mode, ctrl_key(ch)),
                    Some(kk::Action::LegendToggle)
                ),
                "{mode:?} still toggles the legend on C-{ch:?}"
            );
        }
    }
}

#[test]
fn undo_is_bound_to_ctrl_u_alone() {
    assert!(
        matches!(
            action_of(kk::Mode::Edit, ctrl_key('u')),
            Some(kk::Action::BufferUndo)
        ),
        "C-u no longer undoes"
    );
    assert!(
        !matches!(
            action_of(kk::Mode::Edit, ctrl_key('/')),
            Some(kk::Action::BufferUndo)
        ),
        "C-/ must not undo any more"
    );
    assert!(
        !matches!(
            action_of(kk::Mode::Edit, code_key(tuinix::KeyCode::Escape)),
            Some(kk::Action::BufferUndo)
        ),
        "Escape must not undo; it toggles the legend"
    );
}

#[test]
fn ctrl_c_quits_the_edit_mode() {
    assert!(matches!(
        action_of(kk::Mode::Edit, ctrl_key('c')),
        Some(kk::Action::Quit)
    ));
}

#[test]
fn ctrl_j_inserts_a_newline_in_the_edit_mode() {
    // `Enter` and `C-j` are the same command in the editor kk's chords come
    // from, so the pair travels together here too.
    assert!(
        matches!(
            action_of(kk::Mode::Edit, ctrl_key('j')),
            Some(kk::Action::NewlineInsert)
        ),
        "C-j no longer inserts a newline"
    );
    assert!(
        matches!(
            action_of(kk::Mode::Edit, code_key(tuinix::KeyCode::Enter)),
            Some(kk::Action::NewlineInsert)
        ),
        "Enter must keep inserting a newline beside C-j"
    );
}

#[test]
fn the_edit_legend_names_the_newline_binding() {
    // The table is kept in step with the bindings by hand, so a binding with
    // no row breaks that promise.
    assert!(
        kk::Mode::Edit
            .legend()
            .iter()
            .any(|row| row.contains("newline")),
        "the edit legend omits the newline binding"
    );
}

#[test]
fn ctrl_j_is_not_a_newline_outside_the_edit_mode() {
    // Only the edit mode has a buffer to split: the search prompt has no
    // newline of its own and the extension mode no text at all.
    for &mode in &[kk::Mode::Search, kk::Mode::Ext] {
        assert!(
            !matches!(
                action_of(mode, ctrl_key('j')),
                Some(kk::Action::NewlineInsert)
            ),
            "{mode:?} must not insert a newline"
        );
    }
}

#[test]
fn the_search_legend_names_enter_where_it_finishes_the_search() {
    // `Enter` is the one command that ends a search, so it has to be the one
    // the legend names. The row spells the chord short, `Ent`, the way the
    // other rows shorten theirs, and labels it with a word like the rest.
    assert!(
        matches!(
            action_of(kk::Mode::Search, code_key(tuinix::KeyCode::Enter)),
            Some(kk::Action::SearchFinish)
        ),
        "Enter no longer finishes the search"
    );
    assert!(
        kk::Mode::Search
            .legend()
            .iter()
            .any(|row| row.contains("Ent")),
        "the search legend omits the Enter binding"
    );
}

#[test]
fn every_other_mode_has_a_way_back_to_edit() {
    // Search and Ext are entered from Edit, so a chord that leaves for Edit is
    // what keeps them from trapping the editor. Search names the two ways out
    // after where the cursor ends up, so both count.
    for &mode in &[kk::Mode::Search, kk::Mode::Ext] {
        let returns = built_in_keys().into_iter().any(|key| {
            matches!(
                mode.resolve(&tuinix::Input::Key(key)),
                Some(kk::Resolved {
                    action: Some(
                        kk::Action::Cancel | kk::Action::SearchCancel | kk::Action::SearchFinish
                    ),
                    mode: Some(kk::Mode::Edit),
                })
            )
        });
        assert!(returns, "{mode:?} cannot return to the edit mode");
    }
}

#[test]
fn the_ext_mode_binds_its_chords_after_ctrl_x() {
    // The buffer-level commands live behind `C-x`, which is the way into the
    // Ext mode. Inside Ext the ctrl prefix is dropped, so each is a plain
    // letter that runs and returns to Edit.
    for (ch, expected) in [('r', 0), ('a', 1), ('e', 2)] {
        let resolved = kk::Mode::Ext
            .resolve(&tuinix::Input::Key(char_key(ch)))
            .unwrap_or_else(|| panic!("{ch} is not bound in Ext"));
        assert_eq!(
            resolved.mode,
            Some(kk::Mode::Edit),
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
fn the_ext_mode_binds_force_save_to_s() {
    let resolved = kk::Mode::Ext
        .resolve(&tuinix::Input::Key(char_key('s')))
        .expect("s is bound in Ext");
    assert_eq!(resolved.mode, Some(kk::Mode::Edit));
    assert!(
        matches!(resolved.action, Some(kk::Action::BufferForceSave)),
        "s force-saves inside Ext: {:?}",
        resolved.action
    );

    // The upper-case letter used to be the force-save and is now unbound, so a
    // stray shift is no longer a destructive command.
    assert!(
        kk::Mode::Ext
            .resolve(&tuinix::Input::Key(char_key('S')))
            .is_none(),
        "S must not be bound in Ext any more"
    );
}

#[test]
fn tab_saves_in_the_edit_mode_without_a_mode_switch() {
    let resolved = kk::Mode::Edit
        .resolve(&tuinix::Input::Key(code_key(tuinix::KeyCode::Tab)))
        .expect("Tab is bound in Edit");
    assert_eq!(
        resolved.mode, None,
        "Tab must not change mode; saving is not Ext's alone"
    );
    assert!(
        matches!(resolved.action, Some(kk::Action::BufferSave)),
        "Tab saves: {:?}",
        resolved.action
    );
}

#[test]
fn alt_matches_nothing_in_every_mode() {
    // kk binds no `M-` chord, so an Alt chord is unbound: it must not fall
    // back to the same chord without Alt (`M-r` is not plain `r`, and `M-C-r`
    // is not `C-r`).
    for &mode in &MODES {
        for ch in ['r', '<', '>', 'm', 'l', 'w', 'g', 'z', 'c'] {
            assert!(
                action_of(mode, alt_key(ch)).is_none(),
                "M-{ch} resolves to something in {mode:?}"
            );
            assert!(
                action_of(mode, alt_ctrl_key(ch)).is_none(),
                "M-C-{ch} resolves to something in {mode:?}"
            );
        }
    }
}

#[test]
fn an_alt_chord_is_not_inserted_as_text() {
    // The guard is on Alt alone, so `M-z` is inert rather than inserting `z`.
    assert!(action_of(kk::Mode::Edit, alt_key('z')).is_none());
}

#[test]
fn an_alt_arrow_matches_nothing() {
    // Alt arrows today fall through to the ctrl-free arrow arm and move the
    // cursor. The guard drops them like every other Alt chord; the reach is
    // deliberate, not a side effect to be special-cased away.
    for code in [
        tuinix::KeyCode::Up,
        tuinix::KeyCode::Down,
        tuinix::KeyCode::Left,
        tuinix::KeyCode::Right,
    ] {
        assert!(
            kk::Mode::Edit
                .resolve(&tuinix::Input::Key(alt_code_key(code)))
                .is_none(),
            "M-{code:?} resolves to something in Edit"
        );
    }
}

#[test]
fn each_mode_resolves_its_own_chords() {
    // Both modes bind `C-k`, but each keeps its own clipboard: the two
    // actions differ, so the cut never lands in the other's clipboard.
    assert!(matches!(
        action_of(kk::Mode::Edit, ctrl_key('k')),
        Some(kk::Action::LineCutTail)
    ));
    assert!(matches!(
        action_of(kk::Mode::Search, ctrl_key('k')),
        Some(kk::Action::SearchCutQuery)
    ));

    // Both modes bind `Tab`, but it means something different in each: saving
    // in Edit and moving to the next hit in Search.
    assert!(matches!(
        action_of(kk::Mode::Edit, code_key(tuinix::KeyCode::Tab)),
        Some(kk::Action::BufferSave)
    ));
    assert!(matches!(
        action_of(kk::Mode::Search, code_key(tuinix::KeyCode::Tab)),
        Some(kk::Action::SearchNextHit)
    ));

    // Search has its own keys, which Edit does not.
    assert!(action_of(kk::Mode::Search, ctrl_key('r')).is_some());
    assert!(action_of(kk::Mode::Edit, ctrl_key('r')).is_none());
}

#[test]
fn non_key_input_never_resolves() {
    for &mode in &MODES {
        assert!(mode.resolve(&tuinix::Input::Mouse(left_press())).is_none());
        assert!(
            mode.resolve(&tuinix::Input::Unrecognized {
                bytes: b"\x1b[?".to_vec()
            })
            .is_none()
        );
        assert!(
            mode.resolve(&tuinix::Input::Paste {
                bytes: b"hi".to_vec()
            })
            .is_none()
        );
    }
}

#[test]
fn input_display_covers_unrecognized_and_paste() {
    let unrecognized = kk::display_input(&tuinix::Input::Unrecognized {
        bytes: b"\x1b[?".to_vec(),
    });
    assert_eq!(unrecognized, "<UNRECOGNIZED>");

    let paste = kk::display_input(&tuinix::Input::Paste {
        bytes: b"hi".to_vec(),
    });
    assert_eq!(paste, "<PASTE>");
}

#[test]
fn input_display_renders_keys_and_mouse() {
    assert_eq!(kk::display_input(&tuinix::Input::Key(char_key('a'))), "a");
    assert_eq!(
        kk::display_input(&tuinix::Input::Key(tuinix::KeyInput {
            ctrl: true,
            alt: false,
            code: tuinix::KeyCode::Char('a'),
        })),
        "C-a"
    );
    // Alt is spelled rather than folded away: `M-a` must not render as the
    // unbindable-looking `a`, and `M-C-a` must not render as the bound `C-a`.
    assert_eq!(kk::display_input(&tuinix::Input::Key(alt_key('a'))), "M-a");
    assert_eq!(
        kk::display_input(&tuinix::Input::Key(alt_ctrl_key('a'))),
        "C-M-a"
    );
    assert_eq!(
        kk::display_input(&tuinix::Input::Mouse(left_press())),
        "<LEFTCLICK>"
    );
}
