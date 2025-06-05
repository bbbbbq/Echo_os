#![no_std]

// Architecture-specific modules
#[cfg(target_arch = "riscv64")]
pub mod riscv64_qemu;

#[cfg(target_arch = "loongarch64")]
pub mod loongarch64_qemu;

// Re-export the appropriate architecture module based on the target architecture
#[cfg(target_arch = "riscv64")]
pub use riscv64_qemu as target;

#[cfg(target_arch = "loongarch64")]
pub use loongarch64_qemu as target;
