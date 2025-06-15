use crate::executor::error::TaskError;
use crate::user_handler::handler::UserHandler;
use alloc::vec;
use config::target::plat::PAGE_SIZE;
use alloc::string::ToString;
use frame::alloc_continues;
use log::debug;
use mem::memregion::MemRegion;
use mem::memregion::MemRegionType;
use mem::pagetable::PageTable;
use memory_addr::VirtAddrRange;
use memory_addr::align_up;
use memory_addr::{MemoryAddr, PageIter, PhysAddr, VirtAddr};
use page_table_multiarch::MappingFlags;
use page_table_multiarch::PageSize;

impl UserHandler {
    pub async fn sys_brk(&mut self, addr: usize) -> Result<usize, TaskError> {
        // if addr != 0 {
        //     get_boot_page_table().change_pagetable();
        //     let mut heap = self.task.get_heap();
        //     let old_end = heap.get_end();
        //     debug!("sys_brk @ new: {:#x} old: {:#x}", old_end + addr, old_end);
        //     let new_end = heap.sbrk(addr, &mut self.task.page_table.lock());
        //     self.task.set_heap(heap);
        //     self.task.page_table.lock().change_pagetable();
        //     Ok(new_end)
        // }
        // else {
        //     Ok(self.task.get_heap().get_end())
        // }
        let mut heap = self.task.get_heap();
        heap.virt_range.end = VirtAddr::from_usize(heap.virt_range.end.as_usize() + addr);
        self.task.set_heap(heap);
        Ok(heap.virt_range.end.as_usize())
    }

    pub async fn sys_mmap(
        &mut self,
        addr: usize,
        len: usize,
        prot: usize,
        flags: usize,
        fd: usize,
        offset: usize,
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_mmap @ addr: {:#x}, len: {:#x}, prot: {:#x}, flags: {:#x}, fd: {:#x}, offset: {:#x}",
            addr, len, prot, flags, fd, offset
        );

        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        let file_size = file.get_file_size()?;
        let len = if len == 0 { file_size } else { len };
        let aligned_len = align_up(len, PAGE_SIZE);
        let start_vaddr = if addr == 0 {
            self.task
                .pcb
                .lock()
                .mem_set
                .find_free_area(aligned_len)
                .as_usize()
        } else {
            addr
        };
        let end_vaddr = start_vaddr + aligned_len;

        let frame_tracers = alloc_continues(aligned_len / PAGE_SIZE);
        if frame_tracers.is_empty() {
            return Err(TaskError::ENOMEM);
        }
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(frame_tracers[0].paddr.as_usize() as *mut u8, aligned_len)
        };
        file.read_at(offset, buffer)?;
        let start_paddr = frame_tracers[0].paddr;
        let end_paddr = start_paddr + aligned_len;
        let mut mem_region = MemRegion::new_mapped(
            start_vaddr.into(),
            end_vaddr.into(),
            start_paddr,
            end_paddr,
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE,
            "mmap".to_string(),
            MemRegionType::MMAP,
        );
        self.task
            .page_table
            .lock()
            .map_region_user(&mut mem_region)
            .map_err(|_| TaskError::EFAULT)?;

        self.task.pcb.lock().mem_set.push_region(mem_region);
        Ok(start_vaddr)
    }
}
