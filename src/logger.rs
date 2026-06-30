pub fn init_logger(verbose: u8) {
    use log::LevelFilter;

    let level = match verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter_level(level)
        .parse_default_env()
        .init();
}
