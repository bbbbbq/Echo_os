use crate::user_handler::handler::UserHandler;
use log::debug;
use crate::executor::error::TaskError;


impl UserHandler {
    pub async fn sys_exit(&self, exit_code: isize) -> Result<usize, TaskError> {
        debug!("sys_exit @ exit_code: {}  task_id: {:?}", exit_code, self.tid);
        self.task.thread_exit(exit_code as _);
        Ok(0)
    }
}