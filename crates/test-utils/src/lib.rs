use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
pub mod forgejo;
pub fn setup_test_logger() {
    let fmt_layer = fmt::layer().with_target(false).with_level(true);

    let reg = tracing_subscriber::registry().with(LevelFilter::from_level(Level::DEBUG));

    let result = reg.with(fmt_layer.pretty()).try_init();

    match result {
        Ok(_) => tracing::info!("Tracing subscriber initialized"),
        Err(_) => {
            // Optionally log a warning or just ignore
            tracing::warn!("Tracing subscriber was already initialized, skipping reinit");
        }
    }
}
