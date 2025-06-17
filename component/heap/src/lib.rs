#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;

use alloc::string::ToString;
use config::riscv64_qemu::plat::PAGE_SIZE;
use buddy_system_allocator::LockedHeap;
use config::target::plat::HEAP_SIZE;
use console::println;
use memory_addr::{PageIter4K, VirtAddr};
use core::ptr;
use log::{debug, info};
use mem::memregion::{MemRegion, MemRegionType};
use mem::pagetable::PageTable;
use memory_addr::MemoryAddr;
use memory_addr::VirtAddrRange;
use page_table_multiarch::MappingFlags;

// 堆空间
#[unsafe(link_section = ".bss.heap")]
static mut HEAP_SPACE: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// 堆内存分配器
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<30> = LockedHeap::empty();

/// 初始化堆内存分配器
pub fn init() {
    unsafe {
        // Get the heap memory address using raw pointer operations
        let heap_start = ptr::addr_of_mut!(HEAP_SPACE) as usize;

        // Initialize the allocator with the address and size
        HEAP_ALLOCATOR.lock().init(heap_start, HEAP_SIZE);

        info!(
            "kernel HEAP init: {:#x} - {:#x}  size: {:#x}",
            heap_start,
            heap_start + HEAP_SIZE,
            HEAP_SIZE
        );
    }
}

/// Allocation error handler
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout)
}

#[derive(Debug)]
#[derive(Clone, Copy)]
pub struct HeapUser {
    pub virt_range: VirtAddrRange,
}

impl HeapUser {
    pub fn new(virt_range: VirtAddrRange) -> Self {
        assert!(
            virt_range.start < virt_range.end,
            "Virtual address range start must be less than end"
        );
        assert!(
            virt_range.start.is_aligned_4k(),
            "Virtual address start must be 4K aligned"
        );

        Self { virt_range }
    }

    pub fn convert_to_memregion(&self) -> MemRegion {
        MemRegion::new_anonymous(
            self.virt_range.start,
            self.virt_range.end,
            MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
            "user_heap".to_string(),
            MemRegionType::HEAP,
        )
    }

    pub fn get_end(&self) -> usize {
        self.virt_range.end.as_usize()
    }

    pub fn sbrk(&mut self, increment: usize,pagetable:&mut PageTable) -> usize{
        let pages = increment.div_ceil(PAGE_SIZE.try_into().unwrap());
        let new_end = self.virt_range.end.add(pages * PAGE_SIZE);
        let old_end = self.virt_range.end;
        let mut new_region = MemRegion::new_anonymous(
            old_end,
            new_end,
            MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
            "user_heap_sbrk".to_string(),
            MemRegionType::HEAP,
        );
        pagetable.map_region_user_frame(&mut new_region);
        self.virt_range.end = new_end;
        new_end.as_usize()
    }

    pub fn map(&mut self, pagetable: &mut PageTable) -> bool {
        let vaddr_range = self.virt_range;
        debug!("vaddr start: {:?}, vaddr end: {:?}", vaddr_range.start, vaddr_range.end);
        let page_iter = PageIter4K::new(vaddr_range.start, vaddr_range.end).expect("Failed to create PageIter");
        for page in page_iter {
            if pagetable.translate(page).is_none() {
                let mut region = MemRegion::new_anonymous(
                    page,
                    VirtAddr::from_usize(page.as_usize() + PAGE_SIZE),
                    MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
                    "user_heap_on_demand_map".to_string(),
                    MemRegionType::HEAP,
                );
                pagetable.map_region_user_frame(&mut region);
            }
        }
        true
    }
}
