#![no_std]

//! 架构抽象(arch)模块
//!
//! 提供多架构下的通用接口与导出。

#[cfg(target_arch = "loongarch64")]
mod loongarch64;
#[cfg(target_arch = "riscv64")]
mod riscv64;

#[cfg(target_arch = "loongarch64")]
/// 导出 loongarch64 架构下的相关内容。
pub use loongarch64::*;
#[cfg(target_arch = "riscv64")]
/// 导出 riscv64 架构下的相关内容。
pub use riscv64::*;
