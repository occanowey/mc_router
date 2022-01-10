use std::io::ErrorKind;

use fern::colors::{Color, ColoredLevelConfig};

pub fn setup() -> Result<(), fern::InitError> {
    match std::fs::create_dir("logs/") {
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
        _ => Ok(()),
    }
    .expect("Failed to create `./logs/` directory.");

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Cyan);

    let file_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::DateBased::new("logs/", "%Y-%m.log"));

    let stdout_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout());

    fern::Dispatch::new()
        .chain(file_logger)
        .chain(stdout_logger)
        .apply()?;

    Ok(())
}
