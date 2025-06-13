use crate::user_handler::handler::UserHandler;
use memory_addr::VirtAddr;

use crate::executor::error::TaskError;

impl UserHandler
{
    pub async fn sys_write(&self, fd: usize, buf_ptr: VirtAddr, count: usize) -> Result<usize, TaskError> {
        let buffer = unsafe { core::slice::from_raw_parts(buf_ptr.as_ptr(), count) };
        let file = self.task.get_fd(fd).expect("invalid fd");
        Ok(file.write_at(buffer)?)
    }
}