use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::runtime;

struct UnrealLogger;

static LOGGER: UnrealLogger = UnrealLogger;

impl Log for UnrealLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) || !runtime::api::is_api_initialized() {
            return;
        }

        let level = match record.level() {
            Level::Error => runtime::LOG_ERROR,
            Level::Warn => runtime::LOG_WARNING,
            Level::Info | Level::Debug | Level::Trace => runtime::LOG_DISPLAY,
        };

        runtime::ulog!(level, "{}", record.args());
    }

    fn flush(&self) {}
}

pub(crate) fn install() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
