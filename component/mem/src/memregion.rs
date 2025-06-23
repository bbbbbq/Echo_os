use crate::maptrace::MapMemTrace;
use alloc::string::String;
use config::target::plat::PAGE_SIZE;
use memory_addr::{MemoryAddr, PageIter4K, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;

use super::pagetable::PageTable;
use alloc::sync::Arc;
use alloc::vec::Vec;
use frame::{FrameTracer, alloc_continues};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemRegionType {
    ELF,
    Text,
    BSS,
    RODATA,
    DATA,
    Mapped,
    STACK,
    HEAP,
    ANONYMOUS,
    MMAP,
    SHARED,
}

/// Memory region
#[derive(Clone, Debug)]
pub struct MemRegion {
    pub range: VirtAddrRange,
    pub map_traces: Vec<MapMemTrace>,
    pub region_type: MemRegionType,
    pub map_flags: MappingFlags,
}

impl MemRegion {
    pub fn new_mapped(
        vaddr_range: VirtAddrRange,
        paddr_range: PhysAddrRange,
        map_flags: MappingFlags,
    ) -> Self {
        assert!(
            vaddr_range.start.align_offset_4k() == 0,
            "vaddr_range start is not 4K aligned"
        );
        assert!(
            vaddr_range.end.align_offset_4k() == 0,
            "vaddr_range end is not 4K aligned"
        );
        assert!(
            paddr_range.start.align_offset_4k() == 0,
            "paddr_range start is not 4K aligned"
        );
        assert!(
            paddr_range.end.align_offset_4k() == 0,
            "paddr_range end is not 4K aligned"
        );
        let pages = (vaddr_range.end - vaddr_range.start) / PAGE_SIZE;
        assert!(
            vaddr_range.size() == paddr_range.size(),
            "vaddr_range and paddr_range sizes don't match"
        );

        let mut map_traces = Vec::with_capacity(pages);
        let mut vaddr = vaddr_range.start;
        let mut paddr = paddr_range.start;

        for _ in 0..pages {
            let frame = FrameTracer::new(paddr);
            map_traces.push(MapMemTrace::new(vaddr, Arc::new(frame), map_flags));
            vaddr += PAGE_SIZE;
            paddr += PAGE_SIZE;
        }

        Self {
            range: vaddr_range,
            map_traces,
            region_type: MemRegionType::Mapped,
            map_flags,
        }
    }

    pub fn new_anonymous(
        vaddr_range: VirtAddrRange,
        map_flags: MappingFlags,
        mem_type: MemRegionType,
    ) -> Self {
        assert!(
            vaddr_range.start.align_offset_4k() == 0,
            "vaddr_range start is not 4K aligned"
        );
        assert!(
            vaddr_range.end.align_offset_4k() == 0,
            "vaddr_range end is not 4K aligned"
        );

        let pages = (vaddr_range.end - vaddr_range.start) / PAGE_SIZE;
        let frames = alloc_continues(pages);
        assert!(frames.len() == pages);
        let mem_map_trace: Vec<MapMemTrace> = frames
            .iter()
            .map(|frame| MapMemTrace::new(vaddr_range.start, Arc::new(frame.clone()), map_flags))
            .collect();
        Self {
            range: vaddr_range,
            map_traces: mem_map_trace,
            region_type: mem_type,
            map_flags,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map_traces.is_empty()
    }

    pub fn sub_region(&mut self, start: usize, size: usize) {
        let vaddr_range = VirtAddrRange::new(VirtAddr::from(start), VirtAddr::from(start + size));
        // 移除落在指定虚拟地址区间内的映射记录
        self.map_traces
            .retain(|trace| !(trace.vaddr >= vaddr_range.start && trace.vaddr < vaddr_range.end));
    }
}
