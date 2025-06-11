use super::memregion::MemRegion;
use alloc::vec::Vec;
#[derive(Clone)]
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

impl core::fmt::Debug for MemSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MemSet")
            .field("regions", &self.regions)
            .finish()
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
}
