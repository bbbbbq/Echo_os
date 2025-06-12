use log::warn;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::thread::UserTask;
use super::handler::UserHandler;
use crate::executor::task::AsyncTask;
use crate::executor::ops::yield_now;
use trap::trapframe::TrapFrame;
use async_recursion::async_recursion;
use alloc::boxed::Box;
use log::info;

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


impl UserHandler
{
    pub async fn entry_point(&mut self, _cx_ref: &mut TrapFrame) {
        loop {
            // info!("entry_point loop");
            yield_now().await;
        }
    }
}