use alloc::string::String;
use memory_addr::{MemoryAddr, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;
use super::pagetable::PageTable;
use alloc::vec::Vec;
use frame::FrameTracer;

#[derive(Debug,Clone, Copy)]
pub enum MemRegionType {
    ELF,
    Text,
    BSS,
    RODATA,
    DATA,
    STACK,
    HEAP,
    ANONYMOUS,
    MMAP,
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
    pub frames: Option<Vec<FrameTracer>>,
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
            .field("frames", &self.frames)
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
            frames: None,
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
            frames: None,
        }
    }
    pub fn map_user_frame(&mut self, page_table: &mut PageTable) {
        page_table.map_region_user_frame(self);
    }

    pub fn sub_region(&self, start_vaddr: usize, size:usize) -> (Self, Self)
    {
        let end_vaddr = start_vaddr + size;
        let region_start = self.vaddr_range.start.as_usize();
        let region_end = self.vaddr_range.end.as_usize();
        
        assert!(region_start <= start_vaddr && end_vaddr <= region_end, 
                "sub_region range must be within the original region");
        
        let left = Self {
            vaddr_range: VirtAddrRange::new(
                VirtAddr::from(region_start),
                VirtAddr::from(start_vaddr),
            ),
            paddr_range: self.paddr_range.map(|range| {
                let offset = start_vaddr - region_start;
                PhysAddrRange::new(
                    range.start,
                    range.start.add(offset),
                )
            }),
            pte_flags: self.pte_flags,
            name: self.name.clone(),
            region_type: self.region_type,
            is_mapped: self.is_mapped,
            frames: None,
        };
        
        let right = Self {
            vaddr_range: VirtAddrRange::new(
                VirtAddr::from(end_vaddr),
                VirtAddr::from(region_end),
            ),
            paddr_range: self.paddr_range.map(|range| {
                let offset = end_vaddr - region_start;
                PhysAddrRange::new(
                    range.start.add(offset),
                    range.end,
                )
            }),
            pte_flags: self.pte_flags,
            name: self.name.clone(),
            region_type: self.region_type,
            is_mapped: self.is_mapped,
            frames: None,
        };
        
        (left, right)
    }
}





