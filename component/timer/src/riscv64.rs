use core::time::Duration;
use riscv::register::time;

//! RISC-V 64 架构下的定时器实现。
//!
//! 提供定时器初始化、超时设置、时间获取等功能，基于 SBI 调用。

/// 定时器时钟频率（单位：Hz）。
const CLOCK_FREQ: u64 = 10_000_000;
/// 定时器中断间隔（单位：毫秒）。
const TIMER_INTERVAL_MS: u64 = 10;
/// 每毫秒对应的时钟周期数。
const TICKS_PER_MS: u64 = CLOCK_FREQ / 1000;
/// 每次定时器中断的时钟周期数。
const TIMER_INTERVAL_TICKS: u64 = TIMER_INTERVAL_MS * TICKS_PER_MS;

/// 初始化 RISC-V 定时器。
///
/// # 安全性
/// 该函数会修改 SIE 寄存器并设置下一个定时器超时。
pub fn init() {
    unsafe {
        riscv::register::sie::set_stimer();
    }
    set_next_timeout();
    log::info!(
        "RISC-V timer initialized using SBI. CLOCK_FREQ: {} Hz, Interval: {} ms",
        CLOCK_FREQ,
        TIMER_INTERVAL_MS
    );
}

/// 设置下一个定时器超时。
///
/// 该函数会读取当前时钟周期数，并通过 SBI 设置下一个定时器触发时间。
/// 如果设置失败，会输出错误日志。
pub fn set_next_timeout() {
    let current_ticks = time::read() as u64;
    let next_trigger_ticks = current_ticks + TIMER_INTERVAL_TICKS;
    let ret = sbi_rt::set_timer(next_trigger_ticks);
    if ret.error == 0 {
    } else {
        log::error!(
            "Failed to set SBI timer: error code {}. SBI call returned value: {}",
            ret.error,
            ret.value
        );
    }
}

/// 获取当前时间（单位：毫秒）。
///
/// # 返回值
/// 返回自启动以来的毫秒数。
pub fn get_time_ms() -> u64 {
    (time::read() as u64) / TICKS_PER_MS
}

/// 获取当前时间（[`core::time::Duration`]）。
///
/// # 返回值
/// 返回自启动以来的时间，单位为 [`core::time::Duration`]。
pub fn get_time() -> Duration {
    let ticks = time::read() as u64;
    let nanos = (ticks as u128 * 1_000_000_000) / CLOCK_FREQ as u128;
    Duration::from_nanos(nanos as u64)
}

/// 获取定时器时钟频率（单位：Hz）。
///
/// # 返回值
/// 返回定时器的时钟频率。
pub fn get_clock_freq() -> u64 {
    CLOCK_FREQ
}
