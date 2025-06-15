use page_table_multiarch::PagingHandler;
use memory_addr::{PhysAddr, VirtAddr};
use log::debug;

pub struct PagingHandlerImpl;


impl PagingHandler for PagingHandlerImpl {
    fn alloc_frame() -> Option<PhysAddr> {
        let paddr = frame::alloc_frame().map(|ft| ft.paddr).unwrap();
        // if paddr.as_usize() < VIRT_ADDR_START {
        //     paddr = PhysAddr::from_usize(paddr.as_usize() + VIRT_ADDR_START);
        // }
        debug!("PagingHandler Allocated frame at address: 0x{:x}", paddr.as_usize());
        Some(paddr)
    }

    fn dealloc_frame(paddr: PhysAddr) {
        let frame_to_dealloc = frame::FrameTracer { paddr };
        frame::dealloc_frame(frame_to_dealloc);
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        VirtAddr::from(paddr.as_usize())
    }
}