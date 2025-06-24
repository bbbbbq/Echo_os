use core::slice::from_raw_parts_mut;

use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;
use crate::user_handler::handler::UserHandler;
use alloc::string::ToString;
use config::target::plat::PAGE_SIZE;
use filesystem::file::File;
use frame::alloc_continues;
use frame::dealloc_continues;
use log::debug;
use mem::memregion::MemRegion;
use mem::memregion::MemRegionType;
use memory_addr::MemoryAddr;
use crate::executor::task::AsyncTask;
use memory_addr::align_up;
use memory_addr::VirtAddr;
use page_table_multiarch::MappingFlags;

//!
//! 内存管理相关系统调用实现。
//!
//! 提供 brk/mmap/munmap 等内存分配与映射的异步系统调用实现。
impl UserHandler {
    /// 设置或获取进程数据段末尾（brk 系统调用）。
    ///
    /// # 参数
    /// - `addr`: 新的 brk 地址（0 表示获取当前 brk）
    ///
    /// # 返回值
    /// 返回新的 brk 地址。
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

    /// 内存映射（mmap 系统调用）。
    ///
    /// # 参数
    /// - `addr`: 建议的映射起始地址
    /// - `len`: 映射长度
    /// - `prot`: 保护标志
    /// - `flags`: 映射标志
    /// - `fd`: 文件描述符
    /// - `offset`: 文件偏移
    ///
    /// # 返回值
    /// 返回映射的起始虚拟地址。
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

        // mmap flags from linux uapi
        const PROT_READ: usize = 0x1;
        const PROT_WRITE: usize = 0x2;
        const PROT_EXEC: usize = 0x4;
        const MAP_ANONYMOUS: usize = 0x20;

        if len == 0 {
            return Err(TaskError::EINVAL);
        }
        let aligned_len = align_up(len, PAGE_SIZE);

        let start_vaddr = if addr == 0 {
            self.task.get_last_free_addr(aligned_len)
        } else {
            if addr % PAGE_SIZE != 0 {
                return Err(TaskError::EINVAL);
            }
            VirtAddr::from(addr)
        };
        let end_vaddr = start_vaddr + aligned_len;

        let mut mapping_flags = MappingFlags::USER;
        if prot & PROT_READ != 0 {
            mapping_flags |= MappingFlags::READ;
        }
        if prot & PROT_WRITE != 0 {
            mapping_flags |= MappingFlags::WRITE;
        }
        if prot & PROT_EXEC != 0 {
            mapping_flags |= MappingFlags::EXECUTE;
        }

        let frame_tracers = alloc_continues(aligned_len / PAGE_SIZE);
        if frame_tracers.is_empty() {
            return Err(TaskError::ENOMEM);
        }
        let start_paddr = frame_tracers[0].paddr;

        let mut mem_region = MemRegion::new_mapped(
            start_vaddr,
            end_vaddr,
            start_paddr,
            start_paddr + aligned_len, // end_paddr
            mapping_flags,
            "mmap".to_string(),
            MemRegionType::MMAP,
        );
        mem_region.frames = Some(frame_tracers);

        self.task
            .page_table
            .lock()
            .map_region_user(&mut mem_region)
            .map_err(|()| {
                if let Some(frames) = mem_region.frames.take() {
                    if !frames.is_empty() {
                        dealloc_continues(frames[0], frames.len());
                    }
                }
                TaskError::EFAULT
            })?;

        let buffer = unsafe {
            core::slice::from_raw_parts_mut(start_vaddr.as_mut_ptr(), aligned_len)
        };

        if flags & MAP_ANONYMOUS == 0 {
            if offset % PAGE_SIZE != 0 {
                self.task.pcb.lock().mem_set.unmap_region(start_vaddr.as_usize(), aligned_len, &mut self.task.page_table.lock());
                return Err(TaskError::EINVAL);
            }
            let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
            let read_len = file.read_at(offset, buffer)?;
            if read_len < aligned_len {
                buffer[read_len..].fill(0);
            }
        } else {
            buffer.fill(0);
        }

        self.task.pcb.lock().mem_set.push_region(mem_region);
        Ok(start_vaddr.as_usize())
    }

    /// 解除内存映射（munmap 系统调用）。
    ///
    /// # 参数
    /// - `start`: 映射起始地址
    /// - `len`: 映射长度
    ///
    /// # 返回值
    /// 返回 0 表示成功。
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
