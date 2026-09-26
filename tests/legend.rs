//! Properties and examples of the key-binding legend.

mod helpers_frame;

use helpers_frame::row_text;

// The box strokes this test file expects to see. They are spelled here rather
// than taken from the crate, so the assertions below state the look the legend
// must have instead of agreeing with whatever the crate happens to define.
const VERTICAL: &str = "\u{2502}";
const BOTTOM_LEFT: &str = "\u{2514}";
const HORIZONTAL: &str = "\u{2500}";

/// The size the legend of `context` needs in an ample frame.
fn full_size(context: kk::Context) -> kk::LegendSize {
    kk::legend_size(
        context,
        tuinix::Size {
            rows: 60,
            cols: 200,
        },
    )
}

/// A frame exactly the size of `context`'s legend.
fn exact_frame(context: kk::Context) -> tuinix::Frame {
    let size = full_size(context);
    tuinix::Frame::new(tuinix::Size {
        rows: size.rows,
        cols: size.cols,
    })
}

#[test]
fn a_frame_too_small_for_the_legend_shows_nothing() {
    let mut frame = tuinix::Frame::new(tuinix::Size { rows: 1, cols: 1 });
    kk::LegendRenderer.render(kk::Context::Edit, &mut frame);
    assert_eq!(row_text(&frame, 0, 1), " ");
}

#[test]
fn a_frame_one_row_short_of_the_legend_shows_nothing() {
    let size = full_size(kk::Context::Ext);
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows - 1,
        cols: size.cols,
    });
    kk::LegendRenderer.render(kk::Context::Ext, &mut frame);
    for row in 0..size.rows - 1 {
        assert_eq!(row_text(&frame, row, size.cols).trim(), "");
    }
}

#[test]
fn a_frame_one_column_short_of_the_legend_shows_nothing() {
    let size = full_size(kk::Context::Ext);
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows,
        cols: size.cols - 1,
    });
    kk::LegendRenderer.render(kk::Context::Ext, &mut frame);
    for row in 0..size.rows {
        assert_eq!(row_text(&frame, row, size.cols - 1).trim(), "");
    }
}

#[test]
fn the_ext_legend_fills_a_frame_exactly_its_size() {
    let mut frame = exact_frame(kk::Context::Ext);
    let cols = frame.size().cols;
    kk::LegendRenderer.render(kk::Context::Ext, &mut frame);
    assert_eq!(row_text(&frame, 0, cols), "\u{2502}C-g cancel    ");
    assert_eq!(row_text(&frame, 1, cols), "\u{2502}s   save      ");
    assert_eq!(row_text(&frame, 2, cols), "\u{2502}S   force-save");
    assert_eq!(row_text(&frame, 3, cols), "\u{2502}r   reload    ");
    assert_eq!(row_text(&frame, 4, cols), "\u{2502}a   bof       ");
    assert_eq!(row_text(&frame, 5, cols), "\u{2502}e   eof       ");
    assert_eq!(
        row_text(&frame, 6, cols),
        "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
    );
}

#[test]
fn the_edit_legend_is_exactly_this_text() {
    let mut frame = exact_frame(kk::Context::Edit);
    let cols = frame.size().cols;
    kk::LegendRenderer.render(kk::Context::Edit, &mut frame);

    // The very same strings `kk::legend` lists, with the box painted around
    // them; a row that disagrees with the table is a rendering bug.
    let expected = [
        "\u{2502}C-c quit    ",
        "\u{2502}C-g cancel  ",
        "\u{2502}C-x ext     ",
        "\u{2502}C-s search  ",
        "\u{2502}C-  mark    ",
        "\u{2502}C-w cut     ",
        "\u{2502}C-k cut-line",
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
        "\u{2514}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}",
    ];

    assert_eq!(expected.len(), kk::legend(kk::Context::Edit).len());
    for (row, line) in expected.iter().enumerate() {
        assert_eq!(row_text(&frame, row, cols), *line, "row {row}");
    }
}

#[test]
fn the_search_legend_is_exactly_this_text() {
    let mut frame = exact_frame(kk::Context::Search);
    let cols = frame.size().cols;
    kk::LegendRenderer.render(kk::Context::Search, &mut frame);

    let expected = [
        "\u{2502}C-g cancel    ",
        "\u{2502}C-r \u{21e4} prev-hit",
        "\u{2502}C-s \u{21e5} next-hit",
        "\u{2502}C-y paste     ",
        "\u{2502}C-k cut-line  ",
        "\u{2502}C-a bol       ",
        "\u{2502}C-e eol       ",
        "\u{2502}C-b \u{2190} left    ",
        "\u{2502}C-f \u{2192} right   ",
        "\u{2502}C-h \u{232b} bs      ",
        "\u{2502}C-d \u{2326} delete  ",
        "\u{2514}\u{2500}\u{2500}\u{2500}\u{2500} Esc \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
    ];

    assert_eq!(expected.len(), kk::legend(kk::Context::Search).len());
    for (row, line) in expected.iter().enumerate() {
        assert_eq!(row_text(&frame, row, cols), *line, "row {row}");
    }
}

#[test]
fn every_row_of_the_legend_holds_exactly_one_chord_and_one_label() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let rows = kk::legend(context);
        assert!(!rows.is_empty(), "{context:?} has an empty legend");
        for row in rows {
            let (key, label) = row.trim().split_once(' ').expect("a chord and a label");
            assert!(!key.is_empty(), "{context:?} has an unnamed key in {row:?}");
            assert!(
                !label.trim().is_empty(),
                "{context:?} has an unlabelled key {key:?}"
            );
        }
    }
}

#[test]
fn the_legend_height_counts_every_row_including_the_bottom_border() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        assert_eq!(
            full_size(context).rows,
            kk::legend(context).len(),
            "{context:?}"
        );
    }
}

#[test]
fn the_legend_width_is_the_widest_row() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let widest = kk::legend(context)
            .iter()
            .map(|row| kk::str_cols(row))
            .max()
            .unwrap_or(0);
        assert_eq!(full_size(context).cols, widest, "{context:?}");
    }
}

#[test]
fn every_row_of_the_box_is_the_same_width() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let size = full_size(context);
        let mut frame = exact_frame(context);
        kk::LegendRenderer.render(context, &mut frame);

        for row in 0..size.rows {
            let text = row_text(&frame, row, size.cols);
            assert_eq!(
                kk::str_cols(&text),
                size.cols,
                "{context:?} row {row}: {text:?}"
            );
        }
    }
}

#[test]
fn every_row_paints_its_table_row_and_never_closes_with_a_border() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let size = full_size(context);
        let mut frame = exact_frame(context);
        kk::LegendRenderer.render(context, &mut frame);

        for (row, table_row) in kk::legend(context).iter().enumerate() {
            let text = row_text(&frame, row, size.cols);
            assert!(
                text.starts_with(table_row),
                "{context:?} row {row} does not open with its table row: {text:?}"
            );
            assert!(
                text[table_row.len()..].trim().is_empty(),
                "{context:?} row {row} paints past its table row: {text:?}"
            );
        }
    }
}

#[test]
fn the_binding_rows_open_with_the_left_border() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let rows = kk::legend(context);
        // The last row is the bottom border, which opens with its own corner.
        for (row, binding) in rows[..rows.len() - 1].iter().enumerate() {
            assert!(
                binding.starts_with(VERTICAL),
                "{context:?} row {row} has no left border: {binding:?}"
            );
            assert!(
                !binding.ends_with(VERTICAL),
                "{context:?} row {row} closes with a border: {binding:?}"
            );
        }
    }
}

#[test]
fn the_bottom_border_opens_with_its_own_corner() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let rows = kk::legend(context);
        let bottom = rows.last().expect("a legend has a bottom border");
        assert!(
            bottom.starts_with(BOTTOM_LEFT),
            "{context:?} bottom border has no corner: {bottom:?}"
        );
        assert!(
            bottom.ends_with(HORIZONTAL),
            "{context:?} bottom border does not close with a dash: {bottom:?}"
        );
    }
}

#[test]
fn the_legend_is_painted_against_the_right_edge() {
    let size = full_size(kk::Context::Ext);
    let cols = size.cols + 7;
    let mut frame = tuinix::Frame::new(tuinix::Size {
        rows: size.rows + 3,
        cols,
    });
    kk::LegendRenderer.render(kk::Context::Ext, &mut frame);

    let mut exact = exact_frame(kk::Context::Ext);
    kk::LegendRenderer.render(kk::Context::Ext, &mut exact);
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
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let size = full_size(context);
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

        kk::LegendRenderer.render(context, &mut frame);

        let left = size.cols + 4 - size.cols;
        for row in 0..size.rows {
            let text = row_text(&frame, row, size.cols + 4);
            assert!(
                !text[left..].contains('.'),
                "{context:?} row {row} leaks the frame underneath: {text:?}"
            );
        }
    }
}

#[test]
fn a_wider_frame_does_not_change_the_legend() {
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let size = full_size(context);
        let mut narrow = exact_frame(context);
        let mut wide = tuinix::Frame::new(tuinix::Size {
            rows: size.rows,
            cols: size.cols + 30,
        });
        kk::LegendRenderer.render(context, &mut narrow);
        kk::LegendRenderer.render(context, &mut wide);

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
    for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
        let full = full_size(context);
        for cols in [full.cols, full.cols - 1, 1, 0] {
            let size = kk::legend_size(context, tuinix::Size { rows: 60, cols });
            assert_eq!(size.cols, cols.min(full.cols), "{context:?} cols {cols}");
        }
        for rows in [full.rows, full.rows - 1, 1, 0] {
            let size = kk::legend_size(context, tuinix::Size { rows, cols: 200 });
            assert_eq!(size.rows, rows.min(full.rows), "{context:?} rows {rows}");
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

        for context in [kk::Context::Edit, kk::Context::Search, kk::Context::Ext] {
            kk::LegendRenderer.render(context, &mut frame);
        }

        for (position, _) in frame.chars() {
            assert!(position.row < rows, "row {} outside {rows}", position.row);
            assert!(position.col < cols, "col {} outside {cols}", position.col);
        }
        Ok(())
    })?;
    Ok(())
}
