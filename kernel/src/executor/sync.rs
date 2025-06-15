
use crate::executor::id_alloc::TaskId;
use crate::executor::thread::UserTask;
use alloc::sync::Arc;
use timer::get_time;
use core::future::{self, Future};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;

pub struct WaitPid(pub Arc<UserTask>, pub isize);

impl Future for WaitPid {
    type Output = Result<Arc<UserTask>, TaskError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let inner = self.0.pcb.lock();
        let res = inner
            .children
            .iter()
            .find(|x| (self.1 == -1 || x.task_id == TaskId(self.1 as usize)) && x.exit_code().is_some())
            .cloned();
        drop(inner);
        match res {
            Some(task) => Poll::Ready(Ok(task.clone())),
            None => Poll::Pending,
        }
    }
}

pub struct Sleep {
    pub time:Duration,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ns = get_time();

        match ns > self.time {
            true => Poll::Ready(()),
            false => Poll::Pending,
        }
    }
}