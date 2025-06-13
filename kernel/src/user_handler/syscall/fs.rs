use crate::user_handler::handler::UserHandler;
use memory_addr::VirtAddr;

use crate::executor::error::TaskError;
use crate::alloc::string::ToString;
use filesystem::path::Path;
use log::debug;
impl UserHandler
{
    pub async fn sys_write(&self, fd: usize, buf_ptr: VirtAddr, count: usize) -> Result<usize, TaskError> {
        let buffer = unsafe { core::slice::from_raw_parts(buf_ptr.as_ptr(), count) };
        let file = self.task.get_fd(fd).expect("invalid fd");
        Ok(file.write_at(buffer)?)
    }

    pub async fn chdir(&mut self, path: &str) -> Result<usize, TaskError> {
        self.task.pcb.lock().curr_dir = Path::new(path.to_string()).into();
        Ok(0)
    }

    pub async fn sys_mkdirat(&mut self, dirfd: isize, path: &str, mode: usize) -> Result<usize, TaskError> {
        debug!(
            "sys_mkdirat @ dirfd: {}, path: {}, mode: {}",
            dirfd, path, mode
        );
        
        let dir = self.task.get_fd(dirfd as usize).ok_or(TaskError::NotFound)?;
        
        dir.mkdir_at(path)?;
        Ok(0)
    }
    
    pub async fn sys_close(&mut self, fd: usize) -> Result<usize, TaskError> {
        self.task.pcb.lock().fd_table.close(fd);
        Ok(0)
    }
}