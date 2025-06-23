#![no_std]

//! 控制台(console)模块
//!
//! 提供基本的输出、日志打印、ANSI颜色支持等功能。

#[cfg(target_arch = "riscv64")]
pub mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

use log::{Level, Log, Metadata, Record};
use log::{LevelFilter, set_logger, set_max_level};

/// 无标准库环境下的日志实现。
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
        let mut writer = riscv64::DebugWriter;

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

/// 打印一行到控制台，自动换行。
///
/// # 用法
/// ```
/// println!("Hello, world!");
/// println!("num = {}", 42);
/// ```
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::riscv64::DebugWriter;
        let _ = writeln!(writer, $($arg)*);
    })
}

/// 打印到控制台，不自动换行。
///
/// # 用法
/// ```
/// print!("Hello");
/// print!(" world\n");
/// ```
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = $crate::riscv64::DebugWriter;
        let _ = write!(writer, $($arg)*);
    })
}

/// 初始化控制台日志系统。
///
/// 设置日志级别并注册日志输出。
pub fn init() {
    static LOGGER: NoStdLogger = NoStdLogger;

    let log_level = LevelFilter::Debug;

    set_logger(&LOGGER).unwrap();
    set_max_level(log_level);

    println!("Log level set to: {:?}", log_level);
}
