use memory_addr::{VirtAddr, VirtAddrRange, MemoryAddr};
use page_table_multiarch::MappingFlags;

pub struct MemRegion {
    pub vaddr_range: VirtAddrRange,
    pub pte_flags: MappingFlags, // Corrected typo: pte_flages -> pte_flags
}

impl MemRegion {
    pub fn new(start_vaddr: VirtAddr, end_vaddr: VirtAddr) -> Self { // Renamed end_addr to end_vaddr for consistency
        assert!(start_vaddr.align_offset_4k() == 0, "start_vaddr must be 4K aligned");
        assert!(end_vaddr.align_offset_4k() == 0, "end_vaddr must be 4K aligned");
        Self {
            vaddr_range: VirtAddrRange::new(start_vaddr, end_vaddr),
            pte_flags: MappingFlags::empty(), // Added missing field, corrected typo, and added comma
        }
    }
}





