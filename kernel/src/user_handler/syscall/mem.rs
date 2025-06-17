use crate::executor::error::TaskError;
use crate::user_handler::handler::UserHandler;
use config::target::plat::PAGE_SIZE;
use alloc::string::ToString;
use frame::alloc_continues;
use log::debug;
use mem::memregion::MemRegion;
use mem::memregion::MemRegionType;
use memory_addr::MemoryAddr;
use crate::executor::task::AsyncTask;
use memory_addr::align_up;
use memory_addr::VirtAddr;
use page_table_multiarch::MappingFlags;

impl UserHandler {
    pub async fn sys_brk(&mut self, addr: usize) -> Result<usize, TaskError> {
        debug!("sys_brk @ addr: {:#x}", addr);
        if addr == 0 {
            let res = self.task.get_heap().get_top();
            return Ok(res);
        }
        self.task.get_heap().brk(addr, &mut self.task.page_table.lock());
        let res = self.task.get_heap().get_top();
        Ok(res)
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
        let map_len = if len == 0 { file_size } else { len };
        let aligned_len = align_up(map_len, PAGE_SIZE);
        let frame_tracers = alloc_continues(aligned_len / PAGE_SIZE);
        let start_vaddr = if addr == 0 {
            let start = self.task.pcb.lock().mem_set.find_free_area(aligned_len);
            start.as_usize()
        } else {
            addr
        };
        let end_vaddr = start_vaddr + aligned_len;
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
            MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
            "mmap".to_string(),
            MemRegionType::MMAP,
        );
        self.task
            .page_table
            .lock()
            .map_region_user(&mut mem_region)
            .map_err(|_| TaskError::EFAULT)?;
        self.task.pcb.lock().mem_set.push_region(mem_region);
        let paddr = self.task.page_table.lock().translate(VirtAddr::from_usize(start_vaddr)).unwrap();
        debug!("sys_mmap @ start_vaddr: {:#x}, paddr: {:#x}", start_vaddr, paddr);
        Ok(start_vaddr)
    }


    pub async fn sys_munmap(&self, start: usize, len: usize) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_munmap @ start: {:#x}, len: {:#x}",
            self.task.get_task_id(),
            start,
            len
        );
        let mut pcb = self.task.pcb.lock();
        let mut page_table = self.task.page_table.lock();
        pcb.mem_set.unmap_region(start, len, &mut page_table);
        Ok(0)
    }

}
