#![no_std]

extern crate alloc;

pub mod memregion;
pub mod memset;
pub mod pag_hal;
pub mod pagetable;

// Define multi-architecture modules and pub use them.
cfg_if::cfg_if! {
    if #[cfg(target_arch = "loongarch64")] {
        use page_table_multiarch::loongarch64::LA64PageTable;
        type OsPageTable<H> = LA64PageTable<H>;
    } else if #[cfg(target_arch = "aarch64")] {
    } else if #[cfg(target_arch = "riscv64")] {
        use page_table_multiarch::riscv::Sv39PageTable;
        type OsPageTable<H> = Sv39PageTable<H>;
    } else if #[cfg(target_arch = "x86_64")] {
    } else {
        compile_error!("unsupported architecture!");
    }
}
