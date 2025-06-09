
use config::target::plat::PAGE_SIZE;
use crate::memset::MemSet;
use page_table_multiarch::riscv::Sv39PageTable;
use page_table_multiarch::{MappingFlags, PagingHandler, PageSize};
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use frame::alloc_continues;
use crate::memregion::MemRegion;
use crate::pag_hal;
use arch::change_pagetable;
use arch::flush;
pub struct PageTable
{
    page_table: Sv39PageTable<pag_hal::PagingHandlerImpl>,
}

impl PageTable
{
    pub fn new() -> Self {
        Self {
            page_table: Sv39PageTable::try_new().expect("Failed to create Sv39PageTable"),
        }
    }

    pub fn map_region_kernel(&mut self, area: MemRegion) {
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

        let _ = self.page_table.map_region(
            start_vaddr,
            get_paddr,
            size,
            area.pte_flags,
            true,
            true
        ).expect("Failed to map region in page table");
    }

    pub fn map_mem_set(&mut self, mem_set: MemSet) {
        for region in mem_set.regions.into_iter() {
            self.map_region_kernel(region);
        }
    }

    pub fn change_pagetable(&self)
    {
        change_pagetable(self.page_table.root_paddr().as_usize())
    }

    pub fn flush()
    {
        flush();
    }
}

