use core::ptr::NonNull;
use frame::{alloc_continues, alloc_frame, dealloc_continues, dealloc_frame, FrameTracer};
use spin::Mutex;
extern crate alloc;
use alloc::vec::Vec;
use config::target::plat::{FRAME_SIZE, PAGE_SIZE, VIRT_ADDR_START};
use memory_addr;
use virtio_drivers::{BufferDirection, Hal, PhysAddr};
static VIRTIO_CONTAINER: Mutex<Vec<FrameTracer>> = Mutex::new(Vec::new());
use log::{debug, trace};
pub struct HalImpl;
use memory_addr::MemoryAddr;

unsafe impl Hal for HalImpl {
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
        let phys_addr_val = base_paddr.as_usize();
        let phys_addr_adjusted = phys_addr_val - VIRT_ADDR_START;

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

    unsafe fn dma_dealloc(_paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        // Convert the usize paddr back to our PhysAddr type
        let paddr_obj = memory_addr::PhysAddr::from_usize(_paddr);
        let frame = FrameTracer::new(paddr_obj);
        let _result = dealloc_continues(frame, pages);
        debug!("dma_dealloc: paddr: 0x{:x}, pages: {}", _paddr, pages);
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        trace!(
            "mmio_phys_to_virt: paddr: {:?}, virt: {:?}",
            paddr,
            (usize::from(paddr) | VIRT_ADDR_START) as *mut u8
        );

        NonNull::new((usize::from(paddr) | VIRT_ADDR_START) as *mut u8).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let raw_ptr = buffer.as_ptr() as *mut u8 as usize;
        // trace!(
        //     "share: raw_ptr: {:#x}, phys: {:#x}",
        //     raw_ptr,
        //     raw_ptr & !VIRT_ADDR_START
        // );

        PhysAddr::from(raw_ptr & !VIRT_ADDR_START)
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}
