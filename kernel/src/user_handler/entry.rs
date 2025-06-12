use super::handler::UserHandler;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::ops::yield_now;
use crate::executor::task::AsyncTask;
use crate::executor::thread::UserTask;
use alloc::boxed::Box;
use async_recursion::async_recursion;
use log::info;
use log::warn;
use trap::trapframe::TrapFrame;
use log::debug;
use crate::user_handler::handler::UserTaskControlFlow;

#[async_recursion(Sync)]
pub async fn user_entry() {
    if let Some(task) = get_cur_usr_task() {
        let cx_ref = task.force_cx_ref();
        let tid = task.get_task_id();
        info!("Starting user task with ID: {:?}", tid);
        UserHandler { task, tid }.entry_point(cx_ref).await;
    } else {
        warn!("user_entry called without a current user task.");
    }
}

impl UserHandler {
    pub async fn entry_point(&mut self, cx_ref: &mut TrapFrame) {
        loop {
            self.check_timer();

            let res = self.handle_syscall(cx_ref);

            if let UserTaskControlFlow::Break = res.await {
                break;
            }

            if let Some(exit_code) = self.check_thread_exit() {
                debug!(
                    "program exit with code: {}  task_id: {:?}  with  inner",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }
            
            // yield_now().await;
        }
    }
}
