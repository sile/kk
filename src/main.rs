//! The `kk` binary: parse arguments and hand the terminal to the I/O edge.
//!
//! The Sans I/O core lives in the `kk` library; the edge (raw mode, the poll
//! loop, and file access) lives in [`app`].
//!
//! Two shapes are decided here: the usual one, where the keyboard is standard
//! input and `FILE` names the buffer, and the pager one, `cat f | kk`, where
//! standard input is the text and the keyboard comes from the controlling
//! terminal instead.

// The edge waits for readiness with `libc::poll`, which needs `unsafe`. That is
// the one call that cannot avoid it, so the binary relaxes the library's
// `forbid` to `deny` and marks that call with `#[expect]`.
#![deny(unsafe_code)]

mod app;

use std::io::{IsTerminal, Read};
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

    // `FILE` may be missing without that being an error by itself: whether it is
    // required is decided below, by whether standard input is a terminal.
    let file: Option<String> = noargs::arg("FILE")
        .example("/path/to/file")
        .doc("A file, optionally followed by :LINE to start at, or :LINE:COLUMN")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let open = plan_open(std::io::stdin().is_terminal(), file.as_deref(), create_new)?;

    match open {
        Open::File { arg, create_new } => {
            let (path, row, col) = split_position(&arg);
            let position = open_position(row, col, tail);
            let app = app::App::new(path, create_new, position)?;
            app.run()?;
        }
        Open::Stdin => {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text)?;
            let tty = std::fs::File::open("/dev/tty")?;
            let driver = tuinix::TerminalDriver::with_input(tty)?;
            let position = open_position(1, 1, tail);
            let app = app::App::from_stdin(text, driver, position)?;
            app.run()?;
        }
    }

    Ok(())
}

/// Where the initial buffer comes from, once the arguments and standard input
/// have been read together.
#[derive(Debug, PartialEq, Eq)]
enum Open {
    /// Open the file the argument names.
    File { arg: String, create_new: bool },

    /// Read standard input as the buffer, taking keys from the terminal.
    Stdin,
}

/// Decides where the initial buffer comes from.
///
/// The two shapes are one decision: standard input is either the keyboard or
/// the text. When it is a terminal, `FILE` names the buffer and must be present;
/// when it is a pipe, the pipe is the buffer and `FILE` -- or the other
/// file-shaped input, `--create-new` -- has nothing to act on and is refused.
/// `--tail` is not part of this decision: it says *where in* the buffer to put
/// the cursor, which both shapes have, so the caller applies it either way.
///
/// The decision is a pure function of the arguments and that one property of
/// standard input, so the rules can be tested without a terminal or a pipe. The
/// message is returned rather than printed, so the caller owns the error and
/// `main` can hand it to the usual `noargs` reporting.
fn plan_open(
    stdin_is_terminal: bool,
    file: Option<&str>,
    create_new: bool,
) -> Result<Open, &'static str> {
    if stdin_is_terminal {
        return match file {
            Some(arg) => Ok(Open::File {
                arg: arg.to_string(),
                create_new,
            }),
            None => Err("no FILE given"),
        };
    }

    if file.is_some() {
        return Err("FILE is given with piped input");
    }
    if create_new {
        return Err("--create-new is given with piped input");
    }
    Ok(Open::Stdin)
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
    use super::{Open, open_position, plan_open, split_position};
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

    /// A terminal on stdin keeps the file shape: `FILE` names the buffer.
    #[test]
    fn a_terminal_takes_the_named_file() {
        assert_eq!(
            plan_open(true, Some("a.txt"), false),
            Ok(Open::File {
                arg: "a.txt".to_string(),
                create_new: false,
            })
        );
        assert_eq!(
            plan_open(true, Some("a.txt"), true),
            Ok(Open::File {
                arg: "a.txt".to_string(),
                create_new: true,
            })
        );
    }

    /// A terminal on stdin still needs a `FILE`, as before the pipe existed.
    #[test]
    fn a_terminal_without_a_file_is_rejected() {
        assert!(plan_open(true, None, false).is_err());
    }

    /// A pipe on stdin is the buffer, and neither file-shaped input fits it.
    #[test]
    fn a_pipe_refuses_the_file_inputs() {
        assert!(plan_open(false, Some("a.txt"), false).is_err());
        assert!(plan_open(false, None, true).is_err());
    }

    /// A pipe with no `FILE` reads standard input.
    #[test]
    fn a_pipe_reads_standard_input() {
        assert_eq!(plan_open(false, None, false), Ok(Open::Stdin));
    }
}
