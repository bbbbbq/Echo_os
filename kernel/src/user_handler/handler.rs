use crate::executor::thread::UserTask;
use crate::executor::id_alloc::TaskId;
use crate::executor::task::AsyncTask;

use alloc::sync::Arc;
use timer::get_time;
use log::info;

//!
//! 用户任务调度与辅助检查。
//!
//! 提供 UserHandler 结构体、任务退出/定时器检查等。

pub enum UserTaskControlFlow {
    /// 继续调度
    Continue,
    /// 终止调度
    Break,
}

/// 用户任务处理器。
pub struct UserHandler {
    /// 当前任务
    pub task: Arc<UserTask>,
    /// 当前任务 ID
    pub tid: TaskId,
}

impl UserHandler {
    /// 检查线程是否应退出。
    pub fn check_thread_exit(&self) -> Option<usize> {
        self.task
            .exit_code()
            .or(self.task.tcb.read().thread_exit_code.map(|x| x as usize))
    }

    /// 检查定时器是否超时。
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

