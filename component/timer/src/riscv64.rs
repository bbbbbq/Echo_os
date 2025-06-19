use core::time::Duration;
use riscv::register::time;

const CLOCK_FREQ: u64 = 10_000_000;
const TIMER_INTERVAL_MS: u64 = 10;
const TICKS_PER_MS: u64 = CLOCK_FREQ / 1000;
const TIMER_INTERVAL_TICKS: u64 = TIMER_INTERVAL_MS * TICKS_PER_MS;

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

pub fn get_time_ms() -> u64 {
    (time::read() as u64) / TICKS_PER_MS
}

pub fn get_time() -> Duration {
    let ticks = time::read() as u64;
    let nanos = (ticks as u128 * 1_000_000_000) / CLOCK_FREQ as u128;
    Duration::from_nanos(nanos as u64)
}

pub fn get_clock_freq() -> u64 {
    CLOCK_FREQ
}


pub fn current_nsec() -> usize {
    let duration = get_time();
    (duration.as_secs() as usize) * 1_000_000_000 + (duration.subsec_nanos() as usize)
}