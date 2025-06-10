use memory_addr::{MemoryAddr, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;

pub struct MemRegion {
    pub start_range: PhysAddrRange,
    pub vaddr_range: VirtAddrRange,
    pub pte_flags: MappingFlags,
}

impl MemRegion {
    pub fn new(start_vaddr: VirtAddr, end_vaddr: VirtAddr, start_paddr: PhysAddr, end_paddr: PhysAddr) -> Self {
        assert!(start_vaddr.align_offset_4k() == 0, "start_vaddr must be 4K aligned");
        assert!(end_vaddr.align_offset_4k() == 0, "end_vaddr must be 4K aligned");
        assert!(start_paddr.align_offset_4k() == 0, "start_paddr must be 4K aligned");
        assert!(end_paddr.align_offset_4k() == 0, "end_paddr must be 4K aligned");
        assert!(start_vaddr < end_vaddr, "start_vaddr must be less than end_vaddr");
        assert!(start_paddr < end_paddr, "start_paddr must be less than end_paddr");
        assert!(
            end_vaddr.as_usize() - start_vaddr.as_usize() == end_paddr.as_usize() - start_paddr.as_usize(),
            "virtual and physical address ranges must have the same size"
        );
        
        Self {
            start_range: PhysAddrRange::new(start_paddr, end_paddr),
            vaddr_range: VirtAddrRange::new(start_vaddr, end_vaddr),
            pte_flags: MappingFlags::empty(),
        }
    }
}





