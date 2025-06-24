use super::handler::UserHandler;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::task::AsyncTask;
use alloc::boxed::Box;
use async_recursion::async_recursion;
use log::info;
use log::warn;

use trap::trapframe::TrapFrame;
use log::debug;
use crate::user_handler::handler::UserTaskControlFlow;

//!
//! 用户任务入口与调度循环。
//!
//! 提供 user_entry 入口和 UserHandler::entry_point 调度主循环。

/// 用户任务异步入口。
///
/// 获取当前用户任务并进入调度循环。
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
    /// 用户任务主调度循环。
    ///
    /// # 参数
    /// - `cx_ref`: 当前 TrapFrame
    ///
    /// # 行为
    /// 处理系统调用、检查退出、定时器等。
    pub async fn entry_point(&mut self, cx_ref: &mut TrapFrame) {
        loop {


            self.check_timer();

            let res = self.handle_syscall(cx_ref).await;

            if let UserTaskControlFlow::Break = res {
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
