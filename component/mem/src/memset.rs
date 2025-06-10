use super::memregion::MemRegion;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct MemSet {
    pub regions: Vec<MemRegion>,
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
}
