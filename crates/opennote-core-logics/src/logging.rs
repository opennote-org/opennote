use tracing_subscriber::{Layer, Registry, filter::LevelFilter, prelude::*};

use opennote_models::configurations::fields::LoggingLevel;

pub type WindowlessLayer = tracing_subscriber::layer::Identity;

pub fn initialize_logger<L>(logging_level: &LoggingLevel, extra_layer: Option<L>)
where
    L: Layer<Registry> + Send + Sync + 'static,
{
    let filter = match logging_level {
        LoggingLevel::Trace => LevelFilter::TRACE,
        LoggingLevel::Debug => LevelFilter::DEBUG,
        LoggingLevel::Info => LevelFilter::INFO,
        LoggingLevel::Warn => LevelFilter::WARN,
        LoggingLevel::Error => LevelFilter::ERROR,
    };

    tracing_subscriber::registry()
        .with(extra_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();
}
