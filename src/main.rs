fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let foo: usize = noargs::opt("foo")
        .default("1")
        .take(&mut args)
        .then(|a| a.value().parse())?;
    let bar: bool = noargs::flag("bar").take(&mut args).is_present();
    let baz: Option<String> = noargs::arg("[BAZ]")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;
    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    println!("foo: {}, bar: {}, baz: {:?}", foo, bar, baz);

    Ok(())
}
