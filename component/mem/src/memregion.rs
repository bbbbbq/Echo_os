use alloc::string::{String, ToString};
use memory_addr::{MemoryAddr, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;
use super::pagetable::PageTable;

#[derive(Debug,Clone, Copy)]
pub enum MemRegionType {
    Text,
    BSS,
    RODATA,
    DATA,
    STACK,
    HEAP,
    ANONYMOUS,
}

/// Memory region
#[derive(Clone)]
pub struct MemRegion {
    pub vaddr_range: VirtAddrRange,
    pub paddr_range: Option<PhysAddrRange>,
    pub pte_flags: MappingFlags,
    pub name: String,
    pub region_type: MemRegionType,
    pub is_mapped: bool,
}

impl core::fmt::Display for MemRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "MemRegion {{\n    name: \"{}\",\n    vaddr: {:?}, சின்னமாகn    paddr: {:?}, சின்னமாகn    type: {:?}, சின்னமாகn    mapped: {}, சின்னமாகn    flags: {:?}\n}}",
            self.name,
            self.vaddr_range,
            self.paddr_range,
            self.region_type,
            self.is_mapped,
            self.pte_flags
        )
    }
}

impl core::fmt::Debug for MemRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemRegion")
            .field("name", &self.name)
            .field("vaddr_range", &self.vaddr_range)
            .field("paddr_range", &self.paddr_range)
            .field("pte_flags", &self.pte_flags)
            .field("region_type", &self.region_type)
            .field("is_mapped", &self.is_mapped)
            .finish()
    }
}

impl MemRegion {
    pub fn new_mapped(
        start_vaddr: VirtAddr,
        end_vaddr: VirtAddr,
        start_paddr: PhysAddr,
        end_paddr: PhysAddr,
        pte_flags: MappingFlags,
        name: String,
        region_type: MemRegionType,
    ) -> Self {
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
            vaddr_range: VirtAddrRange::new(start_vaddr, end_vaddr),
            paddr_range: Some(PhysAddrRange::new(start_paddr, end_paddr)),
            pte_flags,
            name,
            region_type,
            is_mapped: false,
        }
    }

    pub fn new_anonymous(
        start_vaddr: VirtAddr,
        end_vaddr: VirtAddr,
        pte_flags: MappingFlags,
        name: String,
        region_type: MemRegionType,
    ) -> Self {
        assert!(start_vaddr.align_offset_4k() == 0, "start_vaddr must be 4K aligned");
        assert!(end_vaddr.align_offset_4k() == 0, "end_vaddr must be 4K aligned");
        assert!(start_vaddr < end_vaddr, "start_vaddr must be less than end_vaddr");
        Self {
            vaddr_range: VirtAddrRange::new(start_vaddr, end_vaddr),
            paddr_range: None,
            pte_flags,
            name,
            region_type,
            is_mapped: false,
        }
    }
    pub fn map_user_frame(&mut self, page_table: &mut PageTable) {
        page_table.map_region_user_frame(self.clone());
    }
}





