#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;

use alloc::string::ToString;
use buddy_system_allocator::LockedHeap;
use config::riscv64_qemu::plat::PAGE_SIZE;
use config::target::plat::HEAP_SIZE;
use console::println;
use core::ptr;
use log::{debug, info};
use mem::memregion::{MemRegion, MemRegionType};
use mem::pagetable::PageTable;
use memory_addr::MemoryAddr;
use memory_addr::VirtAddrRange;
use memory_addr::{PageIter4K, VirtAddr};
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

#[derive(Debug, Clone, Copy)]
pub struct HeapUser {
    pub virt_range: VirtAddrRange,
    pub cur_heap_ptr: usize,
}

impl HeapUser {
    pub fn new(virt_range: VirtAddrRange) -> Self {
        assert!(
            virt_range.start <= virt_range.end,
            "Virtual address range start must be less than end"
        );
        assert!(
            virt_range.start.is_aligned_4k(),
            "Virtual address start must be 4K aligned"
        );

        Self {
            virt_range,
            cur_heap_ptr: virt_range.start.as_usize(),
        }
    }

    pub fn set_heap_top(&mut self, top: usize) {
        self.virt_range.end = VirtAddr::from_usize(top);
    }

    pub fn get_bottom(&self) -> usize {
        self.virt_range.start.as_usize()
    }

    pub fn get_top(&self) -> usize {
        self.virt_range.end.as_usize()
    }

    pub fn add_frame(&self,pagetable: &mut PageTable)
    {
        let heap_top = self.get_top();
        let start_vaddr = VirtAddr::from_usize(heap_top);
        let end_vaddr = VirtAddr::from_usize(heap_top + PAGE_SIZE);
        let pte_flages = MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE;
        let mut map_region = MemRegion::new_anonymous(start_vaddr, end_vaddr, pte_flages, "user_heap_add_frame".to_string(), MemRegionType::HEAP);
        pagetable.map_region_user_frame(&mut map_region);
    }

    pub fn get_ptr(&self) -> usize
    {
        self.cur_heap_ptr
    }

    pub fn set_ptr(&mut self, ptr: usize)
    {
        self.cur_heap_ptr = ptr;
    }

    pub fn sbrk(&mut self, increment: usize ,pagetable: &mut PageTable)
    {
        let pages = increment.div_ceil(PAGE_SIZE.try_into().unwrap());
        let old_top = self.get_top();
        let new_top = old_top + pages * PAGE_SIZE;
        let ptr_value = old_top + increment;
        self.set_ptr(ptr_value);
        for _ in 0..pages
        {
            self.add_frame(pagetable);
        }
        self.set_heap_top(new_top);
    }

    pub fn brk(&mut self, addr: usize,pagetable: &mut PageTable)
    {
        let heap_bottom = self.get_bottom();
        let heap_top = self.get_top();
        if addr < heap_bottom
        {
            panic!("brk failed");
        }
        if addr >= heap_bottom && addr <= heap_top
        {
            self.set_ptr(addr);
        }
        if addr > heap_top
        {
            self.sbrk(addr - heap_top,pagetable);
        }
    }
}
