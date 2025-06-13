use crate::executor::thread::UserTask;
use crate::executor::id_alloc::TaskId;
use crate::executor::task::AsyncTask;

use alloc::sync::Arc;
use timer::get_time;
use log::info;

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
        self.task
            .exit_code()
            .or(self.task.tcb.read().thread_exit_code.map(|x| x as usize))
    }

    pub fn check_timer(&self) {
        let pcb = self.task.pcb.lock();
        if let Some(timeout) = pcb.time {
            let now = get_time();
            if now >= timeout {
                info!("timer expired");
                loop{};
            }
        }
    }
}

