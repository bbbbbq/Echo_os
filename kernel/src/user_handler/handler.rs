use crate::executor::thread::UserTask;
use crate::executor::id_alloc::TaskId;
use crate::executor::task::AsyncTask;

use alloc::sync::Arc;


pub enum UserTaskControlFlow {
    Continue,
    Break,
}

pub struct UserHandler {
    pub task: Arc<UserTask>,
    pub tid: TaskId,
}

impl UserHandler {
    pub fn check_thread_exit(&self) -> Option<usize> {
        self.task.exit_code()
    }
}
