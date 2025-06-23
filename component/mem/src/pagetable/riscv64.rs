use crate::memregion::MemRegion;
use crate::memregion::MemRegionType;
use crate::memset::MemSet;
use crate::pag_hal;
use alloc::borrow::ToOwned;
use arch::change_pagetable;
use config::target::plat::PAGE_SIZE;
use config::target::plat::USER_STACK_INIT_SIZE;
use config::target::plat::USER_STACK_TOP;
use config::target::plat::VIRT_ADDR_START;
use console::println;
use frame::alloc_continues;
use frame::alloc_frame;
use log::error;
use log::info;
use memory_addr::AddrRange;
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use page_table_entry::riscv::Rv64PTE;
use page_table_multiarch::PageSize;
use page_table_multiarch::{GenericPTE, MappingFlags, riscv::Sv39PageTable};
// Removed duplicate import of MemRegion
use device::get_mmio_start_end;
use frame::get_frame_start_end;
use log::debug;

unsafe extern "C" {
    fn boot_page_table() -> usize;
    fn _end();
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
            page_table: Sv39PageTable::try_new().expect("Failed to create Sv39PageTable"),
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

    pub fn restore(&mut self) -> Result<(), ()> {
        self.release();
        let paddr = unsafe { boot_page_table() };
        let boot_pte_arrary = paddr as *mut [u64; 512];
        let current_pte_arrary = self.page_table.root_paddr().as_usize() as *mut [u64; 512];
        unsafe {
            (*current_pte_arrary)[0x100..].copy_from_slice(&(*boot_pte_arrary)[0x100..]);
            for i in 0..0x100 {
                (*current_pte_arrary)[i] = 0;
            }
        }

        let (start_addr, end_addr) = get_frame_start_end();
        debug!(
            "frame start_addr: {:x}, end_addr: {:x}",
            start_addr, end_addr
        );
        self.map_direct(
            VirtAddr::from_usize(start_addr),
            end_addr - start_addr,
            MappingFlags::READ | MappingFlags::WRITE,
        );

        let (mmio_start, mmio_end) = get_mmio_start_end();
        debug!(
            "mmio start_addr: {:x}, mmio_end: {:x}",
            mmio_start, mmio_end
        );
        let mut mem_region = MemRegion::new_mapped(
            AddrRange::new(
                VirtAddr::from_usize(mmio_start),
                VirtAddr::from_usize(mmio_end),
            ),
            AddrRange::new(
                PhysAddr::from_usize(mmio_start),
                PhysAddr::from_usize(mmio_end),
            ),
            MappingFlags::READ | MappingFlags::WRITE,
        );
        self.map_region_user(&mut mem_region)?;

        Ok(())
    }

    pub fn release(&mut self) {
        let current_pte_array = self.page_table.root_paddr().as_usize() as *mut [u64; 512];
        unsafe {
            for i in 0..512 {
                (*current_pte_array)[i] = 0;
            }
        }
    }

    pub fn change_pagetable(&self) {
        change_pagetable(self.page_table.root_paddr().as_usize())
    }

    pub fn map_region_user(&mut self, region: &mut MemRegion) -> Result<(), ()> {
        //info!("region : {:?}", region);
        let map_traces = region.map_traces.clone();
        for trace in map_traces {
            let _ = self.page_table.map(
                trace.vaddr,
                trace.frame.paddr,
                PageSize::Size4K,
                trace.pte_flags,
            );
        }
        Ok(())
    }

    pub fn flush() {
        arch::flush_tlb();
    }

    pub fn translate(&self, vaddr: VirtAddr) -> Option<(PhysAddr, MappingFlags)> {
        match self.page_table.query(vaddr) {
            Ok((paddr, flags, _page_size)) => Some((paddr, flags)),
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

    pub fn protect_region(&mut self, region: &mut MemRegion, flags: MappingFlags) {
        let map_traces = region.map_traces.clone();
        for trace in map_traces {
            let _ = self
                .page_table
                .protect_region(trace.vaddr, PAGE_SIZE, flags, true);
        }
    }

    pub fn map_direct(&mut self, vaddr_start: VirtAddr, size: usize, flags: MappingFlags) {
        assert!(
            vaddr_start.align_offset_4k() == 0,
            "vaddr_start is not 4K aligned"
        );
        assert!(size % PAGE_SIZE == 0, "size is not 4K aligned");
        assert!(
            (vaddr_start + size).align_offset_4k() == 0,
            "vaddr_end is not 4K aligned"
        );

        let pages = size.div_ceil(PAGE_SIZE);
        let _ = self.page_table.map_region(
            vaddr_start,
            |vaddr| {
                let p = if vaddr.as_usize() >= VIRT_ADDR_START {
                    vaddr.as_usize() - VIRT_ADDR_START
                } else {
                    vaddr.as_usize()
                };
                PhysAddr::from(p)
            },
            pages * PAGE_SIZE,
            flags,
            true,
            true,
        );
    }

    pub fn get_root_addr_arrary(&self) -> &[u64; 512] {
        // SAFETY: 根页表占用固定的 512 个 u64 条目（4KiB），指针转换后解引用为共享只读引用是安全的。
        unsafe { &*(self.page_table.root_paddr().as_usize() as *const [u64; 512]) }
    }

    pub fn get_pte_array(&self,base_addr:PhysAddr) -> &[u64; 512] {
        // SAFETY: 页表同样为 4KiB（512 个 u64），指针转换后解引用为共享只读引用是安全的。
        unsafe { &*(base_addr.as_usize() as *const [u64; 512]) }
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
