use crate::executor::executor::get_cur_usr_task;
use crate::executor::thread::UserTask;
use super::handler::UserHandler;
use crate::executor::task::AsyncTask;
use crate::executor::ops::yield_now;
use trap::trapframe::TrapFrame;
use async_recursion::async_recursion;
use alloc::boxed::Box;

#[async_recursion(Sync)]
pub async fn user_entry() {
    let task = get_cur_usr_task();
    let cx_ref = task.force_cx_ref();
    let tid = task.get_task_id();
        UserHandler { task, tid }.entry_point(cx_ref).await;
}


impl UserHandler
{
    pub async fn entry_point(&mut self, cx_ref: &mut TrapFrame) {
        let mut times: i32 = 0;
    }
}