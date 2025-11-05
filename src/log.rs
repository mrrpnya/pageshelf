use time::{UtcOffset, format_description::parse};
use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{self, time::OffsetTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn setup_logger(level: Level, pretty: bool) {
    // Define time format and offset
    let time_format = parse("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:2]")
        .expect("format string should be valid");

    let time_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let timer: OffsetTime<_> = OffsetTime::new(time_offset, time_format);

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_level(true)
        .with_timer(timer);

    let reg = tracing_subscriber::registry().with(LevelFilter::from_level(level));

    let result = if pretty {
        reg.with(fmt_layer.pretty()).try_init()
    } else {
        reg.with(fmt_layer.compact()).try_init()
    };

    match result {
        Ok(_) => tracing::info!("Tracing subscriber initialized"),
        Err(_) => {
            // Optionally log a warning or just ignore
            tracing::warn!("Tracing subscriber was already initialized, skipping reinit");
        }
    }
}
