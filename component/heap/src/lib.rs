#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;

use buddy_system_allocator::LockedHeap;
use core::ptr;
use log::info;
use config::target::plat::HEAP_SIZE;

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
        HEAP_ALLOCATOR
            .lock()
            .init(heap_start, HEAP_SIZE);

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
