use config::target::plat::PAGE_SIZE;
use crate::memset::MemSet;
use page_table_multiarch::{MappingFlags, PagingHandler, riscv::Sv39PageTable};
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use frame::alloc_continues;
use crate::memregion::MemRegion;
use crate::pag_hal;
use arch::change_pagetable;
use arch::flush;
use lazy_static::lazy_static;

unsafe extern "C"
{
    pub unsafe fn boot_page_table() -> PhysAddr;
}


lazy_static! {
    pub static ref BOOT_PAGE_TABLE: PageTable = PageTable::new_from_addr(unsafe { boot_page_table() });
}





pub trait PageTableExt
{
    fn new_from_addr(addr: PhysAddr) -> Self;
}

impl PageTableExt for Sv39PageTable<pag_hal::PagingHandlerImpl>
{
    fn new_from_addr(addr: PhysAddr) -> Self {
        #[repr(C)]
        struct TempPageTable {
            root_paddr: PhysAddr,
            _phantom: core::marker::PhantomData<()>,
        }

        let temp_table = TempPageTable {
            root_paddr: addr,
            _phantom: core::marker::PhantomData,
        };

        unsafe { core::mem::transmute(temp_table) }
    }
}

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

    pub fn new_from_addr(addr: PhysAddr) -> Self {
        Self {
            page_table: Sv39PageTable::new_from_addr(addr),
        }
    }

    pub fn restore(&mut self)
    {
       self.page_table = Sv39PageTable::new_from_addr(self.page_table.root_paddr());
    }

    pub fn map_region_user_frame(&mut self, area: MemRegion) {
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

    pub fn map_mem_set_frame(&mut self, mem_set: MemSet) {
        for region in mem_set.regions.into_iter() {
            self.map_region_user_frame(region);
        }
    }

    pub fn map_mem_set_user(&mut self, mem_set: MemSet) {
        for region in mem_set.regions.into_iter() {
            self.map_region_user(region);
        }
    }

    pub fn change_pagetable(&self)
    {
        change_pagetable(self.page_table.root_paddr().as_usize())
    }

    pub fn map_region_user(&mut self, mut region: MemRegion)
    {
        if let Some(paddr_range) = region.paddr_range {
            let start_vaddr = region.vaddr_range.start;
            let size = region.vaddr_range.size();
            let get_paddr = |vaddr: VirtAddr| -> PhysAddr {
                let offset = vaddr.as_usize() - region.vaddr_range.start.as_usize();
                paddr_range.start.add(offset)
            };

            let _ = self.page_table.map_region(
                start_vaddr,
                get_paddr,
                size,
                region.pte_flags,
                true,
                true
            ).expect("Failed to map region in page table");

            region.is_mapped = true;
        }
    }

    pub fn flush()
    {
        flush();
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
            },
            Err(_) => None
        }
    }
}

