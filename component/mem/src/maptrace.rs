use frame::FrameTracer;
use alloc::sync::Arc;
use memory_addr::{PhysAddr, VirtAddr};
use page_table_entry::MappingFlags;

/// MapMemTrace tracks memory mapping information
#[derive(Clone)]
pub struct MapMemTrace {
    pub vaddr: VirtAddr,
    pub frame: Arc<FrameTracer>,
    pub pte_flags: MappingFlags,
}

impl core::fmt::Debug for MapMemTrace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MapMemTrace")
            .field("vaddr", &self.vaddr)
            .field("paddr", &self.frame.paddr)
            .field("flags", &self.pte_flags)
            .finish()
    }
}

impl MapMemTrace {
    pub fn new(vaddr: VirtAddr, frame: Arc<FrameTracer>, pte_flags: MappingFlags) -> Self {
        Self {
            vaddr,
            frame,
            pte_flags,
        }
    }

    pub fn paddr(&self) -> PhysAddr {
        self.frame.paddr
    }

    pub fn vaddr(&self) -> VirtAddr {
        self.vaddr
    }

    pub fn pte_flags(&self) -> MappingFlags {
        self.pte_flags
    }

    pub fn set_vaddr(&mut self, vaddr: VirtAddr) {
        self.vaddr = vaddr;
    }

    pub fn set_pte_flags(&mut self, pte_flags: MappingFlags) {
        self.pte_flags = pte_flags;
    }
}