//! Virtio HAL 层实现
//!
//! 提供物理内存分配、MMIO映射、缓存共享等功能。

use core::ptr::NonNull;
use frame::{FrameTracer, alloc_continues, dealloc_continues};
use spin::Mutex;
extern crate alloc;
use alloc::vec::Vec;
use config::target::plat::VIRT_ADDR_START;
use memory_addr;
use virtio_drivers::{BufferDirection, Hal, PhysAddr};
static VIRTIO_CONTAINER: Mutex<Vec<FrameTracer>> = Mutex::new(Vec::new());
use log::{debug, trace};
pub struct HalImpl;
use memory_addr::MemoryAddr;

/// Virtio HAL trait实现。
unsafe impl Hal for HalImpl {
    /// 分配DMA物理页并返回物理地址和虚拟地址。
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let frames = alloc_continues(pages);
        for i in 0..frames.len() {
            VIRTIO_CONTAINER.lock().push(frames[i].clone());
            assert!(
                frames[i].paddr.is_aligned_4k(),
                "DMA allocation not aligned to 4K boundary"
            );
            trace!("dma alloc frames[{}] : {:?}", i, frames[i]);
        }
        let base_paddr = frames[0].paddr;

        // Convert physical address to usize for virtio
        let mut phys_addr_val = base_paddr.as_usize();
        if phys_addr_val >= VIRT_ADDR_START {
            phys_addr_val -= VIRT_ADDR_START;
        }
        let phys_addr_adjusted = phys_addr_val;

        // Create a virtual address pointer from the physical address
        let vaddr = NonNull::new(phys_addr_val as *mut u8).unwrap();

        debug!(
            "dma_alloc: orig paddr: 0x{:x}, adjusted paddr: 0x{:x}, vaddr: 0x{:x}",
            phys_addr_val,
            phys_addr_adjusted,
            vaddr.as_ptr() as usize
        );

        // Return the adjusted physical address for virtio driver
        (phys_addr_adjusted, vaddr)
    }

    /// 释放DMA物理页。
    unsafe fn dma_dealloc(_paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        // Convert the usize paddr back to our PhysAddr type
        let paddr_obj = memory_addr::PhysAddr::from_usize(_paddr);
        let frame = FrameTracer::new(paddr_obj);
        let _result = dealloc_continues(frame, pages);
        debug!("dma_dealloc: paddr: 0x{:x}, pages: {}", _paddr, pages);
        0
    }

    /// MMIO物理地址转虚拟地址。
    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        trace!(
            "mmio_phys_to_virt: paddr: {:?}, virt: {:?}",
            paddr,
            (usize::from(paddr) | VIRT_ADDR_START) as *mut u8
        );

        NonNull::new((usize::from(paddr) | VIRT_ADDR_START) as *mut u8).unwrap()
    }

    /// 共享缓存区，返回物理地址。
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let raw_ptr = buffer.as_ptr() as *mut u8 as usize;
        // trace!(
        //     "share: raw_ptr: {:#x}, phys: {:#x}",
        //     raw_ptr,
        //     raw_ptr & !VIRT_ADDR_START
        // );

        PhysAddr::from(raw_ptr & !VIRT_ADDR_START)
    }

    /// 取消缓存区共享。
    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}
