use crate::executor::error::TaskError;
use crate::user_handler::handler::UserHandler;
use log::debug;
use crate::executor::task::CloneFlags;
use crate::user_handler::userbuf::UserRef;
impl UserHandler {
    pub async fn sys_exit(&self, exit_code: isize) -> Result<usize, TaskError> {
        debug!(
            "sys_exit @ exit_code: {}  task_id: {:?}",
            exit_code, self.tid
        );
        self.task.thread_exit(exit_code as _);
        Ok(0)
    }

    pub async fn sys_clone(
        &self,
        flags: usize,
        stack: usize,
        ptid: UserRef<u32>,
        tls: usize,
        ctid: UserRef<u32>,
    ) -> Result<usize, TaskError> {
        let sig = flags & 0xff;
        debug!(
            "[task {}] sys_clone @ flags: {:#x}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
            self.tid, flags, stack, ptid, tls, ctid
        );
        let flags = CloneFlags::from_bits_truncate(flags);
        debug!(
            "[task {}] sys_clone @ flags: {:?}, stack: {:#x}, ptid: {}, tls: {:#x}, ctid: {}",
            self.tid, flags, stack, ptid, tls, ctid
        );

    }
}
