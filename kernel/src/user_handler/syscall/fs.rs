use crate::user_handler::handler::UserHandler;
use log::debug;
use memory_addr::VirtAddr;

use crate::executor::error::TaskError;
use log::info;

impl UserHandler
{
    pub async fn sys_write(&self, fd: usize, buf_ptr: VirtAddr, count: usize) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_write @ fd: {} buf_ptr: {:?} count: {}",
            self.tid, fd as isize, buf_ptr, count
        );
        // ********** test **********
        let pagetable = &self.task.page_table;
        let paddr = pagetable.page_table.root_paddr();
        info!("paddr : {:?}", paddr );

        pagetable.print_maped_region();
        

        // ********** test_end **********

        let buffer = unsafe { core::slice::from_raw_parts(buf_ptr.as_ptr(), count) };
        let file = self.task.get_fd(fd).expect("invalid fd");
        Ok(file.write_at(buffer)?)
    }
}