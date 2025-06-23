#![no_std]

//! 启动(boot)模块
//!
//! 提供多架构下的启动入口、启动页表等相关功能。

/// 宏定义模块，包含内核入口宏等。
pub mod macro_def;

#[cfg(target_arch = "loongarch64")]
mod loongarch64;
#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "loongarch64")]
/// 导出 loongarch64 架构下的启动相关内容。
pub use loongarch64::*;
#[cfg(target_arch = "riscv64")]
/// 导出 riscv64 架构下的启动相关内容。
pub use riscv64::*;
