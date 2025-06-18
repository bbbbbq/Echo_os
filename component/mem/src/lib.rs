#![no_std]

extern crate alloc;

pub mod memregion;
pub mod memset;
pub mod pag_hal;
pub mod pagetable;
use memory_addr::{VirtAddr, PhysAddr};
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

pub trait VirtAddrExt {
    fn slice_mut_as_len(&self, len: usize) -> &mut [u8];
    fn get_mut(&self) -> &mut usize;
}

pub trait PhysAddrExt
{
    fn slice_mut_as_len(&self, len: usize) -> &mut [u8];
    fn get_mut(&self) -> &mut usize;
}


impl VirtAddrExt for VirtAddr {
    fn slice_mut_as_len(&self, len: usize) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_usize() as *mut u8, len) }
    }
    fn get_mut(&self) -> &mut usize {
        unsafe { &mut *(self.as_usize() as *mut usize) }
    }
}

impl PhysAddrExt for PhysAddr {
    fn slice_mut_as_len(&self, len: usize) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.as_usize() as *mut u8, len) }
    }
    fn get_mut(&self) -> &mut usize {
        unsafe { &mut *(self.as_usize() as *mut usize) }
    }
}
