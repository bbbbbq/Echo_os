use page_table_multiarch::PagingHandler;
use memory_addr::{PhysAddr, VirtAddr};

pub struct PagingHandlerImpl;


impl PagingHandler for PagingHandlerImpl {
    fn alloc_frame() -> Option<PhysAddr> {
        frame::alloc_frame().map(|ft| ft.paddr)
    }

    fn dealloc_frame(paddr: PhysAddr) {
        let frame_to_dealloc = frame::FrameTracer { paddr };
        frame::dealloc_frame(frame_to_dealloc);
    }

    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr {
        VirtAddr::from(paddr.as_usize())
    }
}