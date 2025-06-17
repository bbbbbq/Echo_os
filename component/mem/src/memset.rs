use crate::pagetable::PageTable;


use super::memregion::MemRegion;
use alloc::vec::Vec;
use memory_addr::{VirtAddr, align_up};

#[derive(Clone, Debug)]
pub struct MemSet {
    pub regions: Vec<MemRegion>,
}

impl core::fmt::Display for MemSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MemSet {{")?;
        for region in &self.regions {
            writeln!(f, "{}", region)?;
        }
        write!(f, "}}")
    }
}

impl MemSet {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn push_region(&mut self, region: MemRegion) {
        self.regions.push(region);
    }

    pub fn get_base(&self) -> usize {
        self.regions
            .iter()
            .map(|r| r.vaddr_range.start.as_usize())
            .min()
            .unwrap_or(0)
    }

    pub fn find_free_area(&self, size: usize) -> VirtAddr {
        // a simple version
        let mut sorted_regions = self.regions.clone();
        sorted_regions.sort_by_key(|x| x.vaddr_range.start);
        let mut last_end = VirtAddr::from(0x1000_0000); // mmap area start
        for region in sorted_regions {
            if last_end.as_usize() + size <= region.vaddr_range.start.as_usize() {
                return last_end;
            }
            last_end = region.vaddr_range.end;
        }
        VirtAddr::from(align_up(last_end.as_usize(), 4096))

        // VirtAddr::from_usize(0x300000000)
    }

    pub fn unmap_region(&mut self, start: usize, size: usize, pagetable: &mut PageTable) {
        if let Some(index) = self.regions.iter().position(|region| {
            region.vaddr_range.start.as_usize() <= start
                && start + size <= region.vaddr_range.end.as_usize()
        }) {
            let target_region = self.regions.remove(index);

            // Unmap from page table
            let _ = pagetable
                .page_table
                .unmap_region(VirtAddr::from(start), size, true)
                .expect("unmap failed");

            let (left, right) = target_region.sub_region(start, size);

            if left.vaddr_range.size() > 0 {
                self.regions.push(left);
            }
            if right.vaddr_range.size() > 0 {
                self.regions.push(right);
            }
        }
    }
}
