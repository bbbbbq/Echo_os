use super::memregion::MemRegion;
use super::PagHal::PagingHandlerImpl;
use crate::OsPageTable;
use alloc::vec::Vec;
use frame::{alloc_frame, dealloc_frame, FrameTracer};
use log::trace;
use memory_addr::PageIter4K; // Import for page iteration
use memory_addr::{PhysAddr, VirtAddr}; // Assuming 'paging' feature enables Page
use page_table_multiarch::{MappingFlags, PageSize, PageTable64, PagingError}; // Use PageTable64 trait

pub struct MemSet {
    regions: Vec<MemRegion>,
    pagetable: OsPageTable<PagingHandlerImpl>,
}

impl MemSet {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            pagetable: OsPageTable::<PagingHandlerImpl>::try_new().unwrap(),
        }
    }

    pub fn push_region(&mut self, region: MemRegion) {
        self.regions.push(region);
    }

    pub fn map_all(&mut self) -> Result<(), PagingError> {
        for region in &self.regions {
            let vaddr_range = region.vaddr_range;
            let flags = region.pte_flags;

            trace!("Mapping region: {:?} with flags: {:?}", vaddr_range, flags);

            let iter_maybe = PageIter4K::new(vaddr_range.start, vaddr_range.end);
            if let Some(actual_iterator) = iter_maybe {
                for vaddr_page in actual_iterator {
                    let paddr = alloc_frame()
                        .expect("Failed to allocate frame for mapping")
                        .paddr;
                    trace!("Mapping vaddr: {:?} to paddr: {:?}", vaddr_page, paddr);
                    self.pagetable
                        .map(vaddr_page, paddr, PageSize::Size4K, flags)?;
                }
            }
        }
        Ok(())
    }

    pub fn unmap_all(&mut self) -> Result<(), PagingError> {
        for region in &self.regions {
            let vaddr_range = region.vaddr_range;
            trace!("Unmapping region: {:?}", vaddr_range);

            let iter_maybe = PageIter4K::new(vaddr_range.start, vaddr_range.end);
            if let Some(actual_iterator) = iter_maybe {
                for vaddr_page in actual_iterator {
                    match self.pagetable.unmap(vaddr_page) {
                        Ok((paddr, _, _)) => {
                            trace!("Unmapped vaddr: {:?} from paddr: {:?}", vaddr_page, paddr);
                            let frame_to_dealloc = FrameTracer::new(paddr);
                            dealloc_frame(frame_to_dealloc);
                        }
                        Err(PagingError::NotMapped) => {
                            trace!("Page {:?} was not mapped, skipping unmap.", vaddr_page);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn page_table_root_paddr(&self) -> PhysAddr {
        self.pagetable.root_paddr()
    }
}
