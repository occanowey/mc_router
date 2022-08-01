use tracing::metadata::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::{self, time::FormatTime},
    prelude::__tracing_subscriber_SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

#[derive(Debug)]
struct ShortTime;

impl FormatTime for ShortTime {
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%y-%m-%d %H:%M:%S"))
    }
}

pub fn setup() -> WorkerGuard {
    let stdout_log = {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        fmt::layer()
            .with_timer(ShortTime)
            .with_target(false)
            .with_filter(filter)
    };

    let (file_log, guard) = {
        let file_appender = tracing_appender::rolling::daily("logs", "router.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        (fmt::layer().json().with_writer(non_blocking), guard)
    };

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(file_log)
        .init();

    guard
}
