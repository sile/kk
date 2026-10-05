//! The `kk` binary: parse arguments and hand the terminal to the I/O edge.
//!
//! The Sans I/O core lives in the `kk` library; the edge (raw mode, the poll
//! loop, and file access) lives in [`app`].

// The edge waits for readiness with `libc::poll`, which needs `unsafe`. That is
// the one call that cannot avoid it, so the binary relaxes the library's
// `forbid` to `deny` and marks that call with `#[expect]`.
#![deny(unsafe_code)]

mod app;

use std::path::PathBuf;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    // Before the positional: a trailing `-c` would otherwise be bound as FILE.
    let create_new = noargs::flag("create-new")
        .short('c')
        .doc("Create the file, which must not already exist")
        .take(&mut args)
        .is_present();

    // Before the positional, like `--create-new`: a trailing `-t` is a flag,
    // not the FILE.
    let tail = noargs::flag("tail")
        .short('t')
        .doc("Open at the end of the file")
        .take(&mut args)
        .is_present();

    let arg: String = noargs::arg("FILE")
        .example("/path/to/file")
        .doc("A file, optionally followed by :LINE to start at, or :LINE:COLUMN")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let (path, row, col) = split_position(&arg);
    let position = open_position(row, col, tail);

    let app = app::App::new(path, create_new, position)?;
    app.run()?;

    Ok(())
}

/// The 0-based position to open at: the parsed one, or the file's end under
/// `--tail`.
///
/// `--tail` is the file end, which is the position a named `:MAX` already
/// reaches: the core clamps a row to the last line and a column to the end of
/// it, so naming `usize::MAX` for both keeps the flag on the one startup path
/// rather than adding a second place to put the cursor.
///
/// The position is 1-based on the command line and 0-based in the core; an
/// out-of-range position is clamped by the core rather than rejected. `--tail`
/// is applied after the shift, on the already-0-based value, so it is exactly
/// `usize::MAX` and not one short.
fn open_position(row: usize, col: usize, tail: bool) -> tuinix::Position {
    if tail {
        return tuinix::Position {
            row: usize::MAX,
            col: usize::MAX,
        };
    }
    tuinix::Position {
        row: row.saturating_sub(1),
        col: col.saturating_sub(1),
    }
}

/// Splits an optional `:LINE[:COLUMN]` off the end of the `FILE` argument.
///
/// The return is the path and the 1-based `row` and `col` named by it, kept
/// apart rather than packed: the two are only ever passed on to `open_position`
/// side by side.
///
/// The suffixes are read off the end, one `:` at a time: the last number is the
/// column, and the number before it the line. Both are 1-based, as the status
/// line spells them, and both default to the start of the file. A `:` that is
/// not followed by digits is part of the path, so `a:b.txt` is left alone, and
/// so is a lone `a.txt:`. The path is not checked against the file system, so
/// this reads the same whether the file exists or `--create-new` is about to
/// make it.
fn split_position(arg: &str) -> (PathBuf, usize, usize) {
    let (path, last) = split_number(arg);
    let (path, first) = split_number(path);
    let (row, col) = match (first, last) {
        // Two numbers: the left one is the line and the right one the column.
        (Some(row), Some(col)) => (row, col),
        // One number: it is a line, and the column is the line's start. A
        // number to the left of it cannot happen, since both are read from the
        // right.
        (None, Some(row)) => (row, 1),
        (Some(_), None) | (None, None) => (1, 1),
    };
    (PathBuf::from(path), row, col)
}

/// Splits a trailing `:NUMBER` off `arg`, returning the rest and the number.
fn split_number(arg: &str) -> (&str, Option<usize>) {
    let Some((rest, digits)) = arg.rsplit_once(':') else {
        return (arg, None);
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return (arg, None);
    }
    // A number too large for `usize` is clamped to the buffer's end, which is
    // where the core puts any out-of-range position.
    (rest, Some(digits.parse().unwrap_or(usize::MAX)))
}

#[cfg(test)]
mod tests {
    use super::{open_position, split_position};
    use std::path::PathBuf;

    /// The path and 1-based position a `FILE` argument parses to.
    fn parsed(arg: &str) -> (PathBuf, usize, usize) {
        split_position(arg)
    }

    #[test]
    fn a_plain_path_starts_at_the_top() {
        assert_eq!(parsed("src/main.rs"), (PathBuf::from("src/main.rs"), 1, 1));
    }

    #[test]
    fn a_trailing_line_names_a_line_only() {
        assert_eq!(
            parsed("src/main.rs:10"),
            (PathBuf::from("src/main.rs"), 10, 1)
        );
    }

    #[test]
    fn a_line_and_a_column_are_read_off_the_end() {
        assert_eq!(
            parsed("src/main.rs:10:5"),
            (PathBuf::from("src/main.rs"), 10, 5)
        );
    }

    #[test]
    fn a_colon_that_is_not_a_position_stays_in_the_path() {
        assert_eq!(parsed("a:b.txt"), (PathBuf::from("a:b.txt"), 1, 1));
        assert_eq!(parsed("a.txt:"), (PathBuf::from("a.txt:"), 1, 1));
        assert_eq!(parsed("a.txt:x"), (PathBuf::from("a.txt:x"), 1, 1));
        assert_eq!(parsed(":10"), (PathBuf::from(""), 10, 1));
    }

    #[test]
    fn only_the_last_two_suffixes_are_a_position() {
        assert_eq!(parsed("a:1:2:3"), (PathBuf::from("a:1"), 2, 3));
    }

    #[test]
    fn a_number_too_large_for_usize_is_clamped() {
        let (path, row, col) = parsed("a:99999999999999999999999999");
        assert_eq!(path, PathBuf::from("a"));
        assert_eq!((row, col), (usize::MAX, 1));
    }

    #[test]
    fn an_ordinary_open_shifts_the_position_to_zero_based() {
        assert_eq!(
            open_position(1, 1, false),
            tuinix::Position { row: 0, col: 0 }
        );
        assert_eq!(
            open_position(10, 5, false),
            tuinix::Position { row: 9, col: 4 }
        );
    }

    #[test]
    fn tail_asks_for_the_file_end_regardless_of_the_position() {
        assert_eq!(
            open_position(1, 1, true),
            tuinix::Position {
                row: usize::MAX,
                col: usize::MAX
            }
        );
        assert_eq!(
            open_position(10, 5, true),
            tuinix::Position {
                row: usize::MAX,
                col: usize::MAX
            }
        );
    }
}
