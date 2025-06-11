use crate::memregion::MemRegion;
use crate::memset::MemSet;
use crate::pag_hal;
use crate::pag_hal::PagingHandlerImpl;
use arch::change_pagetable;
use arch::flush;
use config::target::plat::PAGE_SIZE;
use frame::alloc_continues;
use lazy_static::lazy_static;
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use page_table_multiarch::{MappingFlags, PagingHandler, riscv::Sv39PageTable};

unsafe extern "C" {
    fn boot_page_table() -> usize;
}

pub fn get_boot_page_table() -> PageTable {
    let paddr = unsafe { boot_page_table() };
    PageTable::new_from_addr(PhysAddr::from(paddr))
}

pub struct PageTable {
    page_table: Sv39PageTable<pag_hal::PagingHandlerImpl>,
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


    pub fn restore(&mut self) {
        // 获取启动页表和当前新页表的根页表的物理地址
        let boot_root_paddr = get_boot_page_table().page_table.root_paddr();
        let new_root_paddr = self.page_table.root_paddr();

        // 通过物理地址转虚拟地址，得到可以访问页表内容的裸指针
        let boot_root_ptr = PagingHandlerImpl::phys_to_virt(boot_root_paddr).as_ptr() as *const u64;
        let new_root_ptr = PagingHandlerImpl::phys_to_virt(new_root_paddr).as_mut_ptr() as *mut u64;

        // 将裸指针转换为切片，以便安全访问
        let boot_entries = unsafe { core::slice::from_raw_parts(boot_root_ptr, 512) };
        let new_entries = unsafe { core::slice::from_raw_parts_mut(new_root_ptr, 512) };

        // 复制内核空间的映射（高 256 个条目）
        // 新页表的低 256 个条目（用户空间）保持为空，等待后续映射
        new_entries[256..].copy_from_slice(&boot_entries[256..]);
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

        let _ = self
            .page_table
            .map_region(start_vaddr, get_paddr, size, area.pte_flags, true, true)
            .expect("Failed to map region in page table");
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


    pub fn change_pagetable(&self) {
        change_pagetable(self.page_table.root_paddr().as_usize())
    }


    pub fn map_region_user(&mut self, mut region: MemRegion) {
        if let Some(paddr_range) = region.paddr_range {
            let start_vaddr = region.vaddr_range.start;
            let size = region.vaddr_range.size();
            let get_paddr = |vaddr: VirtAddr| -> PhysAddr {
                let offset = vaddr.as_usize() - region.vaddr_range.start.as_usize();
                paddr_range.start.add(offset)
            };

            let _ = self
                .page_table
                .map_region(start_vaddr, get_paddr, size, region.pte_flags, true, true)
                .expect("Failed to map region in page table");

            region.is_mapped = true;
        }
    }

    pub fn flush() {
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
            }
            Err(_) => None,
        }
    }
}
