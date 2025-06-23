#![no_std]

//! 内存管理(mem)模块
//!
//! 提供内存区域、页表、堆栈等子模块的统一导出。

extern crate alloc;

pub mod memregion;
pub mod memset;
pub mod pag_hal;
pub mod pagetable;
pub mod stack;

// Define multi-architecture modules and pub use them.
cfg_if::cfg_if! {
    if #[cfg(target_arch = "loongarch64")] {
        use page_table_multiarch::loongarch64::LA64PageTable;
        /// 导出LoongArch64架构页表类型。
        type OsPageTable<H> = LA64PageTable<H>;
    } else if #[cfg(target_arch = "aarch64")] {
    } else if #[cfg(target_arch = "riscv64")] {
        use page_table_multiarch::riscv::Sv39PageTable;
        /// 导出RISC-V Sv39架构页表类型。
        type OsPageTable<H> = Sv39PageTable<H>;
    } else if #[cfg(target_arch = "x86_64")] {
    } else {
        compile_error!("unsupported architecture!");
    }
}
