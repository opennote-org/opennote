use opennote_models::configurations::fields::LoggingLevel;

pub fn initialize_logger(logging_level: LoggingLevel) {
    tracing_subscriber::fmt()
        .with_max_level(match logging_level {
            LoggingLevel::Trace => tracing::Level::TRACE,
            LoggingLevel::Debug => tracing::Level::DEBUG,
            LoggingLevel::Info => tracing::Level::INFO,
            LoggingLevel::Warn => tracing::Level::WARN,
            LoggingLevel::Error => tracing::Level::ERROR,
        })
        .init();
}
