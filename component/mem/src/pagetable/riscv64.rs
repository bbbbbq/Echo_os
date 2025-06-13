use crate::memregion::MemRegion;
use crate::memset::MemSet;
use crate::pag_hal;
use crate::pag_hal::PagingHandlerImpl;
use arch::change_pagetable;
use config::target::plat::PAGE_SIZE;
use config::target::plat::VIRT_ADDR_START;
use console::println;
use core::arch::asm;
use frame::alloc_continues;
use lazy_static::lazy_static;
use log::error;
use log::info;
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use page_table_multiarch::{GenericPTE, MappingFlags, PagingHandler, riscv::Sv39PageTable};
use page_table_entry::riscv::Rv64PTE;

unsafe extern "C" {
    fn boot_page_table() -> usize;
}

pub fn get_boot_page_table() -> PageTable {
    let vaddr = unsafe { boot_page_table() };
    // The boot_page_table() returns a virtual address, but SATP needs a physical address.
    // We must convert the VA to a PA before using it.
    let paddr = vaddr - VIRT_ADDR_START;
    PageTable::new_from_addr(PhysAddr::from(paddr))
}

pub struct PageTable {
    pub page_table: Sv39PageTable<pag_hal::PagingHandlerImpl>,
}

impl Clone for PageTable {
    fn clone(&self) -> Self {
        PageTable::new_from_addr(self.page_table.root_paddr())
    }
}

impl PageTable {
    pub fn new() -> Self {
        Self {
            page_table: Sv39PageTable::try_new().expect("Failed to create Sv39PageTable")
        }
    }

    pub fn new_from_addr(addr: PhysAddr) -> Self {
        #[repr(C)]
        struct TempPageTable {
            root_paddr: PhysAddr,
            _phantom: core::marker::PhantomData<()>,
        }

        let temp_table = TempPageTable {
            root_paddr: addr,
            _phantom: core::marker::PhantomData,
        };

        Self {
            page_table: unsafe { core::mem::transmute(temp_table) },
        }
    }

    pub fn restore(&mut self) {
        self.release();
        let mut paddr = unsafe { boot_page_table() };
        if paddr >= VIRT_ADDR_START {
            paddr -= VIRT_ADDR_START;
        }
        let boot_pte_arrary = paddr as *mut [u64; 512];
        let current_pte_arrary = self.page_table.root_paddr().as_usize() as *mut [u64; 512];
        unsafe {
            (*current_pte_arrary)[0x100..].copy_from_slice(&(*boot_pte_arrary)[0x100..]);
            for i in 0..0x100 {
                (*current_pte_arrary)[i] = 0;
            }
        }
    }

    pub fn release(&mut self) {
        let current_pte_array = self.page_table.root_paddr().as_usize() as *mut [u64; 512];
        unsafe {
            for i in 0..512 {
                (*current_pte_array)[i] = 0;
            }
        }
    }

    pub fn map_region_user_frame(&mut self, area: &mut MemRegion) {
        let start_vaddr = area.vaddr_range.start;
        let size = area.vaddr_range.size();
        if PAGE_SIZE == 0 {
            panic!("PAGE_SIZE is zero, division by zero in map_region_kernel");
        }
        let paddr_range = alloc_continues(size / PAGE_SIZE);
        let get_paddr = |vaddr: VirtAddr| -> PhysAddr {
            let offset = vaddr.as_usize() - area.vaddr_range.start.as_usize();
            PhysAddr::from_usize(paddr_range[0].paddr.as_usize() + offset)
        };

        let _ = self
            .page_table
            .map_region(start_vaddr, get_paddr, size, area.pte_flags, true, true)
            .expect("Failed to map region in page table");
        area.is_mapped = true;
    }

    pub fn map_mem_set_frame(&mut self, mem_set: MemSet) {
        for mut region in mem_set.regions.into_iter() {
            self.map_region_user_frame(&mut region);
        }
    }

    pub fn map_mem_set_user(&mut self, mem_set: MemSet) {
        for mut region in mem_set.regions.into_iter() {
            self.map_region_user(&mut region);
        }
    }

    pub fn change_pagetable(&self) {
        change_pagetable(self.page_table.root_paddr().as_usize())
    }

    pub fn map_region_user(&mut self, region: &mut MemRegion) {
        if let Some(paddr_range) = region.paddr_range {
            let start_vaddr = region.vaddr_range.start;
            let size = region.vaddr_range.size();
            let get_paddr = |vaddr: VirtAddr| -> PhysAddr {
                let offset: usize = vaddr.as_usize() - region.vaddr_range.start.as_usize();
                paddr_range.start.add(offset)
            };

            let _ = self
                .page_table
                .map_region(start_vaddr, get_paddr, size, region.pte_flags, true, true)
                .expect("Failed to map region in page table");

            region.is_mapped = true;
        } else {
            error!("Failed to map region in page table");
        }
    }

    pub fn flush() {
        arch::flush();
    }

    pub fn translate(&self, vaddr: VirtAddr) -> Option<PhysAddr> {
        match self.page_table.query(vaddr) {
            Ok((paddr, flags, page_size)) => {
                let is_readable_writable = flags.contains(MappingFlags::READ | MappingFlags::WRITE);
                let is_4k_page = matches!(page_size, page_table_multiarch::PageSize::Size4K);
                if is_readable_writable && is_4k_page {
                    Some(paddr)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn print_maped_region(&self) {
        println!(
            "[kernel] Mapped Regions for PageTable @ {:#x}:",
            self.page_table.root_paddr().as_usize()
        );

        // Inner recursive function to walk the page table
        fn walk(table_paddr: PhysAddr, level: usize, base_va: VirtAddr) {
            let table: &[Rv64PTE] =
                unsafe { core::slice::from_raw_parts(table_paddr.as_usize() as *const _, 512) };

            for (i, pte) in table.iter().enumerate() {
                                if !pte.is_present() {
                    continue;
                }

                // Calculate the size of the region covered by one entry at this level
                let page_size = 1 << (12 + (2 - level) * 9);
                let current_va = base_va + i * page_size;

                                if pte.is_huge() {
                    let page_size_str = ["1G", "2M", "4K"][level];
                    println!(
                        "  VA: {:#x} -> PA: {:#x} (size: {}, flags: {:?})",
                        current_va,
                        pte.paddr(),
                        page_size_str,
                        pte.flags()
                    );
                } else {
                    // It's a pointer to the next level table, so recurse.
                    if level < 2 {
                        walk(pte.paddr(), level + 1, current_va);
                    }
                }
            }
        }

        walk(self.page_table.root_paddr(), 0, VirtAddr::from_usize(0));
        println!("[kernel] --- End of Mapped Regions ---");
    }
}

pub fn change_boot_pagetable() {
    unsafe extern "C" {
        unsafe fn boot_page_table() -> usize;
    }
    let mut paddr = unsafe { boot_page_table() };
    if paddr >= VIRT_ADDR_START {
        paddr -= VIRT_ADDR_START;
    }
    change_pagetable(paddr);
}
