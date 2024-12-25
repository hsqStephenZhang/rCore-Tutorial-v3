/*！

本模块利用 log crate 为你提供了日志功能，使用方式见 main.rs.

*/

use log::{self, Level, LevelFilter, Log, Metadata, Record};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let color = match record.level() {
            Level::Error => 31, // Red
            Level::Warn => 93,  // BrightYellow
            Level::Info => 34,  // Blue
            Level::Debug => 32, // Green
            Level::Trace => 90, // BrightBlack
        };
        println!(
            "\u{1B}[{}m[{:>5}] {}\u{1B}[0m",
            color,
            record.level(),
            record.args(),
        );
    }
    fn flush(&self) {}
}

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("ERROR") | Some("error") => LevelFilter::Error,
        Some("WARN") | Some("warn") => LevelFilter::Warn,
        Some("INFO") | Some("info") => LevelFilter::Info,
        Some("DEBUG") | Some("debug") => LevelFilter::Debug,
        Some("TRACE") | Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}
