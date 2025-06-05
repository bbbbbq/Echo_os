#![no_std]

#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;


use log::{set_logger, set_max_level, LevelFilter};
use log::{Level, Log, Metadata, Record};

struct NoStdLogger;


// ANSI color codes
const COLOR_RED: &str = "\x1b[31m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_BLUE: &str = "\x1b[34m";
const COLOR_MAGENTA: &str = "\x1b[35m";
const COLOR_RESET: &str = "\x1b[0m";

impl Log for NoStdLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        use core::fmt::Write;
        let mut writer = crate::DebugWriter;

        let color = match record.level() {
            Level::Error => COLOR_RED,
            Level::Warn => COLOR_YELLOW,
            Level::Info => COLOR_GREEN,
            Level::Debug => COLOR_BLUE,
            Level::Trace => COLOR_MAGENTA,
        };
        
        let _ = writeln!(
            writer,
            "{}[{}] {}:{} - {}{}",
            color,
            record.level(),
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args(),
            COLOR_RESET
        );
    }

    fn flush(&self) {}
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        let _ = writeln!(writer, $($arg)*);
    })
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::DebugWriter;
        let _ = write!(writer, $($arg)*);
    })
}

pub fn init() {
    static LOGGER: NoStdLogger = NoStdLogger;
    
    let log_level = LevelFilter::Trace;
    
    set_logger(&LOGGER).unwrap();
    set_max_level(log_level);
    
    println!("Log level set to: {:?}", log_level);
}