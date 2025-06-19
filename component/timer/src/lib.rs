#![no_std]

#[macro_use]
extern crate log;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        pub use riscv64::{init, set_next_timeout, get_time_ms, get_time, get_clock_freq, current_nsec};
    } else if #[cfg(target_arch = "aarch64")] {
        // Placeholder for aarch64
        pub fn init() { warn!("Timer for aarch64 not implemented"); }
        pub fn set_next_timeout() { warn!("Timer for aarch64 not implemented"); }
        pub fn get_time_ms() -> u64 { warn!("Timer for aarch64 not implemented"); 0 }
        pub fn get_time() -> core::time::Duration { warn!("Timer for aarch64 not implemented"); core::time::Duration::ZERO }
        pub fn get_clock_freq() -> u64 { warn!("Timer for aarch64 not implemented"); 0 }
    } else if #[cfg(target_arch = "x86_64")] {
        // Placeholder for x86_64
        pub fn init() { warn!("Timer for x86_64 not implemented"); }
        pub fn set_next_timeout() { warn!("Timer for x86_64 not implemented"); }
        pub fn get_time_ms() -> u64 { warn!("Timer for x86_64 not implemented"); 0 }
        pub fn get_time() -> core::time::Duration { warn!("Timer for x86_64 not implemented"); core::time::Duration::ZERO }
        pub fn get_clock_freq() -> u64 { warn!("Timer for x86_64 not implemented"); 0 }
    } else if #[cfg(target_arch = "loongarch64")] {
        // Placeholder for loongarch64
        pub fn init() { warn!("Timer for loongarch64 not implemented"); }
        pub fn set_next_timeout() { warn!("Timer for loongarch64 not implemented"); }
        pub fn get_time_ms() -> u64 { warn!("Timer for loongarch64 not implemented"); 0 }
        pub fn get_time() -> core::time::Duration { warn!("Timer for loongarch64 not implemented"); core::time::Duration::ZERO }
        pub fn get_clock_freq() -> u64 { warn!("Timer for loongarch64 not implemented"); 0 }
    } else {
        compile_error!("Unsupported target_arch for timer component");
    }
}
