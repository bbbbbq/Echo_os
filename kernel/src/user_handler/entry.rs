use super::handler::UserHandler;
use crate::executor::executor::TASK_QUEUE;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::ops::yield_now;
use crate::executor::task::AsyncTask;
use crate::executor::task::AsyncTaskItem;
use alloc::boxed::Box;
use async_recursion::async_recursion;
use log::info;
use log::warn;
use mem::pagetable::change_boot_pagetable;

use crate::user_handler::handler::UserTaskControlFlow;
use log::debug;
use trap::trapframe::TrapFrame;

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
    pub async fn check_signal(&mut self) {
        loop {
            let sig_mask = self.task.tcb.read().sigmask;
            let signal = self
                .task
                .tcb
                .read()
                .signal
                .clone()
                .mask(sig_mask)
                .try_get_signal();
            if let Some(signal) = signal {
                debug!("\ncheck_signal mask: {:?}\n", sig_mask);
                self.handle_signal(signal.clone()).await;
                let mut tcb = self.task.tcb.write();
                tcb.signal.remove_signal(signal.clone());
                // check if it is a real time signal
                if let Some(index) = signal.real_time_index()
                    && tcb.signal_queue[index] > 0
                {
                    tcb.signal.add_signal(signal.clone());
                    tcb.signal_queue[index] -= 1;
                }
                TASK_QUEUE.lock().push_back(AsyncTaskItem {
                    task: self.task.clone(),
                    future: Box::pin(async {}),
                });
                break;
            } else {
                break;
            }
        }
    }

    pub async fn entry_point(&mut self, cx_ref: &mut TrapFrame) {
        let mut times = 0;
        loop {
            // 检查定时器与信号
            self.check_timer();

            match self.handle_syscall(cx_ref).await {
                UserTaskControlFlow::Break => {
                    debug!("[task {:?}] entry_point break", self.task.get_task_id());
                    break;
                }
                UserTaskControlFlow::Continue => {}
            }

           // self.check_signal().await;

            // 再次确认线程是否已经退出
            if let Some(exit_code) = self.check_thread_exit() {
                debug!(
                    "program exit with code: {}  task_id: {:?}  with  inner",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            // 定期让出 CPU，防止长时间独占
            times += 1;
            if times >= 50 {
                times = 0;
                yield_now().await;
            }
        }
        debug!("exit_task: {:?}", self.task.get_task_id());
        change_boot_pagetable();
    }
}
