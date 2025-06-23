#![no_std]

//! 配置(config)模块
//!
//! 提供不同架构下的内核常量与配置项。

// Architecture-specific modules
#[cfg(target_arch = "riscv64")]
pub mod riscv64_qemu;

#[cfg(target_arch = "loongarch64")]
pub mod loongarch64_qemu;

// Re-export the appropriate architecture module based on the target architecture
#[cfg(target_arch = "riscv64")]
/// 导出 riscv64 架构下的配置。
pub use riscv64_qemu as target;

#[cfg(target_arch = "loongarch64")]
/// 导出 loongarch64 架构下的配置。
pub use loongarch64_qemu as target;
