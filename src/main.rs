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

/// Splits an optional `:LINE[:COLUMN]` off the end of the `FILE` argument.
///
/// The suffixes are read off the end, one `:` at a time: the last number is the
/// column, and the number before it the line. Both are 1-based, as the status
/// line spells them. A `:` that is not followed by digits is part of the path,
/// so `a:b.txt` is left alone, and so is a lone `a.txt:`. The path is not
/// checked against the file system, so this reads the same whether the file
/// exists or `--create-new` is about to make it.
fn split_position(arg: &str) -> (PathBuf, Option<(usize, usize)>) {
    let (path, last) = split_number(arg);
    let (path, first) = split_number(path);
    let position = match (first, last) {
        // Two numbers: the left one is the line and the right one the column.
        (Some(row), Some(col)) => Some((row, col)),
        // One number: it is a line, and the column is the line's start. A
        // number to the left of it cannot happen, since both are read from the
        // right.
        (None, Some(row)) => Some((row, 1)),
        (Some(_), None) | (None, None) => None,
    };
    (PathBuf::from(path), position)
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

    let arg: String = noargs::arg("FILE")
        .example("/path/to/file")
        .doc("A file, optionally followed by :LINE to start at, or :LINE:COLUMN")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let (path, position) = split_position(&arg);

    match app::App::new(path, create_new, position) {
        Ok(app) => app.run()?,
        Err(err) => {
            eprintln!("kk: {err}");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::split_position;
    use std::path::PathBuf;

    /// The path and position a `FILE` argument parses to.
    fn parsed(arg: &str) -> (PathBuf, Option<(usize, usize)>) {
        split_position(arg)
    }

    #[test]
    fn a_plain_path_has_no_position() {
        assert_eq!(parsed("src/main.rs"), (PathBuf::from("src/main.rs"), None));
    }

    #[test]
    fn a_trailing_line_names_a_line_only() {
        assert_eq!(
            parsed("src/main.rs:10"),
            (PathBuf::from("src/main.rs"), Some((10, 1)))
        );
    }

    #[test]
    fn a_line_and_a_column_are_read_off_the_end() {
        assert_eq!(
            parsed("src/main.rs:10:5"),
            (PathBuf::from("src/main.rs"), Some((10, 5)))
        );
    }

    #[test]
    fn a_colon_that_is_not_a_position_stays_in_the_path() {
        assert_eq!(parsed("a:b.txt"), (PathBuf::from("a:b.txt"), None));
        assert_eq!(parsed("a.txt:"), (PathBuf::from("a.txt:"), None));
        assert_eq!(parsed("a.txt:x"), (PathBuf::from("a.txt:x"), None));
        assert_eq!(parsed(":10"), (PathBuf::from(""), Some((10, 1))));
    }

    #[test]
    fn only_the_last_two_suffixes_are_a_position() {
        assert_eq!(parsed("a:1:2:3"), (PathBuf::from("a:1"), Some((2, 3))));
    }

    #[test]
    fn a_number_too_large_for_usize_is_clamped() {
        let (path, position) = parsed("a:99999999999999999999999999");
        assert_eq!(path, PathBuf::from("a"));
        assert_eq!(position, Some((usize::MAX, 1)));
    }
}
