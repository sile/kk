//! Properties and examples of the key-binding legend.

mod helpers_frame;

use helpers_frame::row_text;

// The box strokes this test file expects to see. They are spelled here rather
// than taken from the crate, so the assertions below state the look the legend
// must have instead of agreeing with whatever the crate happens to define.
const VERTICAL: &str = "\u{2502}";
const BOTTOM_LEFT: &str = "\u{2514}";
const HORIZONTAL: &str = "\u{2500}";

/// The size the legend of `mode` needs in an ample frame.
fn full_size(mode: kk::Mode) -> tuinix::Size {
    mode.legend_size(tuinix::Size {
        rows: 60,
        cols: 200,
    })
}

/// A frame exactly the size of `mode`'s legend.
fn exact_frame(mode: kk::Mode) -> tuinix::Frame {
    tuinix::Frame::new(full_size(mode))
}

#[test]
fn a_frame_too_small_for_the_legend_shows_nothing() {
    let mut frame = tuinix::Frame::new(tuinix::Size { rows: 1, cols: 1 });
    kk::render_legend(kk::Mode::Edit, &mut frame);
    assert_eq!(row_text(&frame, 0, 1), " ");
}

#[test]
fn a_frame_one_row_short_of_the_legend_shows_nothing() {
    let size = full_size(kk::Mode::Ext);
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows - 1,
        cols: size.cols,
    });
    kk::render_legend(kk::Mode::Ext, &mut frame);
    for row in 0..size.rows - 1 {
        assert_eq!(row_text(&frame, row, size.cols).trim(), "");
    }
}

#[test]
fn a_frame_one_column_short_of_the_legend_shows_nothing() {
    let size = full_size(kk::Mode::Ext);
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows,
        cols: size.cols - 1,
    });
    kk::render_legend(kk::Mode::Ext, &mut frame);
    for row in 0..size.rows {
        assert_eq!(row_text(&frame, row, size.cols - 1).trim(), "");
    }
}

#[test]
fn the_ext_legend_fills_a_frame_exactly_its_size() {
    let mut frame = exact_frame(kk::Mode::Ext);
    let cols = frame.size().cols;
    kk::render_legend(kk::Mode::Ext, &mut frame);
    assert_eq!(row_text(&frame, 0, cols), "\u{2502}C-g cancel    ");
    assert_eq!(row_text(&frame, 1, cols), "\u{2502}s   force-save");
    assert_eq!(row_text(&frame, 2, cols), "\u{2502}r   reload    ");
    assert_eq!(row_text(&frame, 3, cols), "\u{2502}a   bof       ");
    assert_eq!(row_text(&frame, 4, cols), "\u{2502}e   eof       ");
    assert_eq!(
        row_text(&frame, 5, cols),
        "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
    );
}

#[test]
fn the_edit_legend_is_exactly_this_text() {
    let mut frame = exact_frame(kk::Mode::Edit);
    let cols = frame.size().cols;
    kk::render_legend(kk::Mode::Edit, &mut frame);

    // The very same strings `kk::legend` lists, with the box painted around
    // them; a row that disagrees with the table is a rendering bug.
    let expected = [
        "\u{2502}C-c quit    ",
        "\u{2502}C-g cancel  ",
        "\u{2502}C-x ext     ",
        "\u{2502}C-s search  ",
        "\u{2502}C-  mark    ",
        "\u{2502}C-w cut     ",
        "\u{2502}C-k cut-tail",
        "\u{2502}C-y paste   ",
        "\u{2502}C-u undo    ",
        "\u{2502}C-a bol     ",
        "\u{2502}C-e eol     ",
        "\u{2502}C-p \u{2191} up    ",
        "\u{2502}C-n \u{2193} down  ",
        "\u{2502}C-b \u{2190} left  ",
        "\u{2502}C-f \u{2192} right ",
        "\u{2502}C-h \u{232b} bs    ",
        "\u{2502}C-d \u{2326} delete",
        "\u{2502}C-l recenter",
        "\u{2502}Tab \u{21e5} save  ",
        "\u{2514}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}",
    ];

    assert_eq!(expected.len(), kk::Mode::Edit.legend().len());
    for (row, line) in expected.iter().enumerate() {
        assert_eq!(row_text(&frame, row, cols), *line, "row {row}");
    }
}

#[test]
fn the_search_legend_is_exactly_this_text() {
    let mut frame = exact_frame(kk::Mode::Search);
    let cols = frame.size().cols;
    kk::render_legend(kk::Mode::Search, &mut frame);

    let expected = [
        "\u{2502}C-g cancel  ",
        "\u{2502}Ent \u{23ce} finish",
        "\u{2502}C-r \u{21e4} prev  ",
        "\u{2502}C-s \u{21e5} next  ",
        "\u{2502}C-k cut-tail",
        "\u{2502}C-y paste   ",
        "\u{2502}C-a bol     ",
        "\u{2502}C-e eol     ",
        "\u{2502}C-b \u{2190} left  ",
        "\u{2502}C-f \u{2192} right ",
        "\u{2502}C-h \u{232b} bs    ",
        "\u{2502}C-d \u{2326} delete",
        "\u{2514}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}",
    ];

    assert_eq!(expected.len(), kk::Mode::Search.legend().len());
    for (row, line) in expected.iter().enumerate() {
        assert_eq!(row_text(&frame, row, cols), *line, "row {row}");
    }
}

#[test]
fn every_row_of_the_legend_holds_exactly_one_chord_and_one_label() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let rows = mode.legend();
        assert!(!rows.is_empty(), "{mode:?} has an empty legend");
        for row in rows {
            let (key, label) = row.trim().split_once(' ').expect("a chord and a label");
            assert!(!key.is_empty(), "{mode:?} has an unnamed key in {row:?}");
            assert!(
                !label.trim().is_empty(),
                "{mode:?} has an unlabelled key {key:?}"
            );
        }
    }
}

#[test]
fn the_legend_height_counts_every_row_including_the_bottom_border() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        assert_eq!(full_size(mode).rows, mode.legend().len(), "{mode:?}");
    }
}

#[test]
fn the_legend_width_is_the_widest_row() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let widest = mode
            .legend()
            .iter()
            .map(|row| kk::str_cols(row))
            .max()
            .unwrap_or(0);
        assert_eq!(full_size(mode).cols, widest, "{mode:?}");
    }
}

#[test]
fn every_row_of_the_box_is_the_same_width() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let size = full_size(mode);
        let mut frame = exact_frame(mode);
        kk::render_legend(mode, &mut frame);

        for row in 0..size.rows {
            let text = row_text(&frame, row, size.cols);
            assert_eq!(
                kk::str_cols(&text),
                size.cols,
                "{mode:?} row {row}: {text:?}"
            );
        }
    }
}

#[test]
fn every_row_paints_its_table_row_and_never_closes_with_a_border() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let size = full_size(mode);
        let mut frame = exact_frame(mode);
        kk::render_legend(mode, &mut frame);

        for (row, table_row) in mode.legend().iter().enumerate() {
            let text = row_text(&frame, row, size.cols);
            assert!(
                text.starts_with(table_row),
                "{mode:?} row {row} does not open with its table row: {text:?}"
            );
            assert!(
                text[table_row.len()..].trim().is_empty(),
                "{mode:?} row {row} paints past its table row: {text:?}"
            );
        }
    }
}

#[test]
fn the_binding_rows_open_with_the_left_border() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let rows = mode.legend();
        // The last row is the bottom border, which opens with its own corner.
        for (row, binding) in rows[..rows.len() - 1].iter().enumerate() {
            assert!(
                binding.starts_with(VERTICAL),
                "{mode:?} row {row} has no left border: {binding:?}"
            );
            assert!(
                !binding.ends_with(VERTICAL),
                "{mode:?} row {row} closes with a border: {binding:?}"
            );
        }
    }
}

#[test]
fn the_bottom_border_opens_with_its_own_corner() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let rows = mode.legend();
        let bottom = rows.last().expect("a legend has a bottom border");
        assert!(
            bottom.starts_with(BOTTOM_LEFT),
            "{mode:?} bottom border has no corner: {bottom:?}"
        );
        assert!(
            bottom.ends_with(HORIZONTAL),
            "{mode:?} bottom border does not close with a dash: {bottom:?}"
        );
    }
}

#[test]
fn the_legend_is_painted_against_the_right_edge() {
    let size = full_size(kk::Mode::Ext);
    let cols = size.cols + 7;
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows + 3,
        cols,
    });
    kk::render_legend(kk::Mode::Ext, &mut frame);

    let mut exact = exact_frame(kk::Mode::Ext);
    kk::render_legend(kk::Mode::Ext, &mut exact);
    for row in 0..size.rows {
        let text = row_text(&frame, row, cols);
        assert_eq!(
            &text[..7],
            "       ",
            "row {row} does not start with blanks: {text:?}"
        );
        assert_eq!(&text[7..], row_text(&exact, row, size.cols), "row {row}");
    }
}

#[test]
fn the_legend_covers_what_was_painted_under_it() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let size = full_size(mode);
        let mut frame = tuinix::Frame::new(tuinix::Size {
            rows: size.rows + 4,
            cols: size.cols + 4,
        });

        // Fill the whole frame, so any cell the legend leaves unpainted still
        // holds this text and shows through. The filler must not appear in any
        // label, or the check below would read a label as a leak.
        let noise = ".".repeat(size.cols + 4);
        for row in 0..size.rows + 4 {
            kk::put_str(
                &mut frame,
                tuinix::Position { row, col: 0 },
                &noise,
                tuinix::Style::new(),
            );
        }

        kk::render_legend(mode, &mut frame);

        let left = size.cols + 4 - size.cols;
        for row in 0..size.rows {
            let text = row_text(&frame, row, size.cols + 4);
            assert!(
                !text[left..].contains('.'),
                "{mode:?} row {row} leaks the frame underneath: {text:?}"
            );
        }
    }
}

#[test]
fn a_wider_frame_does_not_change_the_legend() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let size = full_size(mode);
        let mut narrow = exact_frame(mode);
        let mut wide = tuinix::Frame::new(tuinix::Size {
            rows: size.rows,
            cols: size.cols + 30,
        });
        kk::render_legend(mode, &mut narrow);
        kk::render_legend(mode, &mut wide);

        for row in 0..size.rows {
            let wide_text = row_text(&wide, row, size.cols + 30);
            assert_eq!(
                &wide_text[size.cols + 30 - size.cols..],
                row_text(&narrow, row, size.cols)
            );
        }
    }
}

#[test]
fn a_legend_clipped_by_its_limit_reports_the_clipped_size() {
    for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
        let full = full_size(mode);
        for cols in [full.cols, full.cols - 1, 1, 0] {
            let size = mode.legend_size(tuinix::Size { rows: 60, cols });
            assert_eq!(size.cols, cols.min(full.cols), "{mode:?} cols {cols}");
        }
        for rows in [full.rows, full.rows - 1, 1, 0] {
            let size = mode.legend_size(tuinix::Size { rows, cols: 200 });
            assert_eq!(size.rows, rows.min(full.rows), "{mode:?} rows {rows}");
        }
    }
}

#[test]
fn the_legend_never_paints_outside_its_frame() -> noprop::TestResult {
    let seed = noprop::seed_from_env_or_time("KK_SEED")?;
    let mut runner = noprop::Runner::new(seed);

    runner.run(256, |ctx| {
        let rows = noprop::sample_usize_in(ctx, 0..=40);
        let cols = noprop::sample_usize_in(ctx, 0..=80);
        let mut frame = tuinix::Frame::new(tuinix::Size { rows, cols });

        for mode in [kk::Mode::Edit, kk::Mode::Search, kk::Mode::Ext] {
            kk::render_legend(mode, &mut frame);
        }

        for (position, _) in frame.chars() {
            assert!(position.row < rows, "row {} outside {rows}", position.row);
            assert!(position.col < cols, "col {} outside {cols}", position.col);
        }
        Ok(())
    })?;
    Ok(())
}
