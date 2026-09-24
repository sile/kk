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

    let path: PathBuf = noargs::arg("FILE")
        .example("/path/to/file")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    match app::App::new(path, create_new) {
        Ok(app) => app.run()?,
        Err(err) => {
            eprintln!("kk: {err}");
            std::process::exit(1);
        }
    }

    Ok(())
}
