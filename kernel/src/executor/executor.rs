use super::task::AsyncTask;
use super::task::AsyncTaskItem;
use alloc::{sync::Arc, vec::Vec};
use arch::get_cur_cpu_id;
use core::sync::atomic::AtomicBool;
use spin::Mutex;

use crate::executor::id_alloc::TaskId;
use alloc::collections::VecDeque;
use alloc::task::Wake;
use core::task::Context;
use core::task::Poll;
use lazy_static::*;

pub(crate) static TASK_QUEUE: Mutex<VecDeque<AsyncTaskItem>> = Mutex::new(VecDeque::new());

pub struct Executor {
    cores: Vec<Mutex<Option<Arc<dyn AsyncTask>>>>,
    is_inited: AtomicBool,
}

pub struct Waker {
    task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {}
}

impl Executor {
    pub fn new() -> Self {
        let cpu_num = arch::get_cpu_num();
        let mut cores = Vec::with_capacity(cpu_num);
        for _ in 0..cpu_num {
            cores.push(Mutex::new(None));
        }
        Self {
            cores,
            is_inited: AtomicBool::new(false),
        }
    }

    pub fn spawn(task: AsyncTaskItem) {
        TASK_QUEUE.lock().push_back(task);
    }

    pub fn run_ready_task(&self) {
        assert!(
            self.is_inited.load(core::sync::atomic::Ordering::Acquire),
            "Executor not initialized"
        );

        let task = TASK_QUEUE.lock().pop_front();
        if let Some(task) = task {
            let AsyncTaskItem { task, mut future } = task;
            task.before_run();
            *self.cores[get_cur_cpu_id()].lock() = Some(task.clone());
            let waker = Arc::new(Waker {
                task_id: task.get_task_id(),
            })
            .into();
            let mut context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task }),
            }
        }
    }
}


pub fn add_ready_task(task: AsyncTaskItem) {
    TASK_QUEUE.lock().push_back(task);
}

pub fn tid2task(tid: TaskId) -> Option<Arc<dyn AsyncTask>> {
    TASK_QUEUE
        .lock()
        .iter()
        .find(|item| item.task.get_task_id() == tid)
        .map(|item| item.task.clone())
}


