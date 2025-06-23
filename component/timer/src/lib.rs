#![no_std]

#[macro_use]
extern crate log;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "riscv64")] {
        mod riscv64;
        pub use riscv64::{init, set_next_timeout, get_time_ms, get_time, get_clock_freq};
    } else if #[cfg(target_arch = "aarch64")] {
        // Placeholder for aarch64
        /// 初始化定时器（AArch64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn init() { warn!("Timer for aarch64 not implemented"); }
        /// 设置下一个定时器超时（AArch64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn set_next_timeout() { warn!("Timer for aarch64 not implemented"); }
        /// 获取当前时间（毫秒，AArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_time_ms() -> u64 { warn!("Timer for aarch64 not implemented"); 0 }
        /// 获取当前时间（Duration，AArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 [`core::time::Duration::ZERO`]，表示未实现。
        pub fn get_time() -> core::time::Duration { warn!("Timer for aarch64 not implemented"); core::time::Duration::ZERO }
        /// 获取定时器时钟频率（AArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_clock_freq() -> u64 { warn!("Timer for aarch64 not implemented"); 0 }
    } else if #[cfg(target_arch = "x86_64")] {
        // Placeholder for x86_64
        /// 初始化定时器（x86_64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn init() { warn!("Timer for x86_64 not implemented"); }
        /// 设置下一个定时器超时（x86_64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn set_next_timeout() { warn!("Timer for x86_64 not implemented"); }
        /// 获取当前时间（毫秒，x86_64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_time_ms() -> u64 { warn!("Timer for x86_64 not implemented"); 0 }
        /// 获取当前时间（Duration，x86_64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 [`core::time::Duration::ZERO`]，表示未实现。
        pub fn get_time() -> core::time::Duration { warn!("Timer for x86_64 not implemented"); core::time::Duration::ZERO }
        /// 获取定时器时钟频率（x86_64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_clock_freq() -> u64 { warn!("Timer for x86_64 not implemented"); 0 }
    } else if #[cfg(target_arch = "loongarch64")] {
        // Placeholder for loongarch64
        /// 初始化定时器（LoongArch64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn init() { warn!("Timer for loongarch64 not implemented"); }
        /// 设置下一个定时器超时（LoongArch64 架构，暂未实现）。
        ///
        /// # 警告
        /// 当前未实现，仅输出警告日志。
        pub fn set_next_timeout() { warn!("Timer for loongarch64 not implemented"); }
        /// 获取当前时间（毫秒，LoongArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_time_ms() -> u64 { warn!("Timer for loongarch64 not implemented"); 0 }
        /// 获取当前时间（Duration，LoongArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 [`core::time::Duration::ZERO`]，表示未实现。
        pub fn get_time() -> core::time::Duration { warn!("Timer for loongarch64 not implemented"); core::time::Duration::ZERO }
        /// 获取定时器时钟频率（LoongArch64 架构，暂未实现）。
        ///
        /// # 返回值
        /// 返回 0，表示未实现。
        pub fn get_clock_freq() -> u64 { warn!("Timer for loongarch64 not implemented"); 0 }
    } else {
        compile_error!("Unsupported target_arch for timer component");
    }
}
