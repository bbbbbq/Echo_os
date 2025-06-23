use crate::pagetable::PageTable;

use super::memregion::MemRegion;
use alloc::vec::Vec;
use config::target::plat::PAGE_SIZE;
use memory_addr::{VirtAddr, VirtAddrRange};

#[derive(Clone)]
pub struct MemSet {
    pub regions: Vec<MemRegion>,
}

impl core::fmt::Display for MemSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MemSet {{")?;
        for (i, region) in self.regions.iter().enumerate() {
            writeln!(f, "Region {}: {:?}", i, region)?;
        }
        write!(f, "}}")
    }
}

impl core::fmt::Debug for MemSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "MemSet {{")?;
        for (i, region) in self.regions.iter().enumerate() {
            writeln!(f, "Region {}: {:?}", i, region)?;
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

    pub fn unmap_region(&mut self, start: usize, size: usize, pagetable: &mut PageTable) {
        // 定义要取消映射的虚拟地址区间
        let vaddr_range = VirtAddrRange::new(VirtAddr::from(start), VirtAddr::from(start + size));

        // 先从页表中取消映射
        let _ = pagetable
            .page_table
            .unmap_region(VirtAddr::from(start), size, true)
            .expect("unmap failed");

        // 遍历所有区域，删除落在区间中的映射记录
        // 反向遍历，方便在遍历过程中安全地移除元素
        let mut idx = 0;
        while idx < self.regions.len() {
            let region = &mut self.regions[idx];

            // 计算当前区域的起止虚拟地址
            let region_start = region
                .map_traces
                .first()
                .map(|t| t.vaddr.as_usize())
                .unwrap_or(0);
            let region_end = region
                .map_traces
                .last()
                .map(|t| t.vaddr.as_usize() + PAGE_SIZE)
                .unwrap_or(0);

            // 只有当要取消的区间完全包含在该区域中时才处理
            if region_start <= start && start + size <= region_end {
                region.sub_region(start, size);

                // 如果该区域已被完全清空，移除它
                if region.is_empty() {
                    self.regions.remove(idx);
                    continue; // 不递增 idx
                }
            }
            idx += 1;
        }
    }

    pub fn find_region(&self, addr: VirtAddr) -> Option<&MemRegion> {
        let addr_usize = addr.as_usize();
        self.regions.iter().find(|region| {
            region.map_traces.iter().any(|trace| {
                let start = trace.vaddr.as_usize();
                let end = start + PAGE_SIZE;
                addr_usize >= start && addr_usize < end
            })
        })
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }
}
