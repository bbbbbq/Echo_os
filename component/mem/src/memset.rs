use super::memregion::MemRegion;
use alloc::vec::Vec;
use memory_addr::{align_up, VirtAddr};

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
            regions: Vec::new()
        }
    }

    pub fn push_region(&mut self, region: MemRegion) {
        self.regions.push(region);
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
    }
}
