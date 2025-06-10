use core::future::Future;
use core::task::{Context, Poll};

use crate::{
    id::TaskId,
    kernel_task::KernelTask,
    task_def::{Task, TaskTrait},
    waker::Waker,
};
use alloc::boxed::Box;
use alloc::{collections::VecDeque, vec::Vec};
use hashbrown::HashMap;
use spin::Mutex;
use alloc::vec;
use alloc::sync::Arc;
use lazy_static::*;


lazy_static! {
    pub static ref TASK_QUEUE: Mutex<VecDeque<Option<Task>>> = Mutex::new(VecDeque::new());
}

lazy_static! {
    pub static ref TASK_HASH_MAP: Mutex<HashMap<TaskId, Arc<dyn TaskTrait>>> =
        Mutex::new(HashMap::new());
}

lazy_static! {
    pub static ref GLOBLE_EXECUTOR: Mutex<Executor> = Mutex::new(Executor::new());
}

pub fn get_task_by_id(id: TaskId) -> Option<Arc<dyn TaskTrait>> {
    let task_queue = TASK_QUEUE.lock();
    for task_option in task_queue.iter() {
        if let Some(task_ref) = task_option {
            if task_ref.task_inner.get_task_id() == id {
                return Some(task_ref.task_inner.clone());
            }
        }
    }
    None
}

pub struct Executor {
    pub cur_task: Vec<Mutex<Option<Arc<dyn TaskTrait>>>>,
}

impl Executor {
    pub fn new() -> Self {
        let cpu_num = arch::get_cpu_num();
        Self {
            cur_task: (0..cpu_num).map(|_| Mutex::new(None)).collect(),
        }
    }

    pub fn run_task(&mut self) {
        loop {
            self.run();
        }
    }

    pub fn run(&mut self) {
        if let Some(Some(owned_task)) = TASK_QUEUE.lock().pop_front() {
            let task_inner = owned_task.task_inner;
            let mut task_future = owned_task.task_future;
            task_inner.before_run();
            *self.cur_task[0].lock() = Some(task_inner.clone());
            let waker = Arc::new(Waker {
                task_id: task_inner.get_task_id(),
            })
            .into();
            let mut context = Context::from_waker(&waker);

            match task_future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => TASK_QUEUE.lock().push_back(Some(Task {
                    task_inner,
                    task_future,
                })),
            }
        }
    }
}

pub fn get_cur_task() -> Option<Arc<dyn TaskTrait>> {
    GLOBLE_EXECUTOR.lock().cur_task[0].lock().clone()
}



pub fn spawn(task: Task, future: impl Future<Output = ()> + Send + Sync + 'static) {
    let task_inner_arc: Arc<dyn TaskTrait + 'static> = task.task_inner; // 从传入的 task 中获取 task_inner
    let task_id = task_inner_arc.get_task_id();

    // 创建一个新的 Task 实例，使用传入的 future
    let task_to_queue = Task {
        task_inner: task_inner_arc.clone(), // 克隆 Arc 以便存储
        task_future: Box::pin(future),      // 将传入的 future 包装成 PinedFuture
    };

    TASK_QUEUE.lock().push_back(Some(task_to_queue));
    TASK_HASH_MAP.lock().insert(task_id, task_inner_arc); // 将 task_inner 存入哈希表
}

pub fn spawn_kernel_task(future: impl Future<Output = ()> + Send + Sync + 'static) {
    let kernel_task_obj = KernelTask::new();
    let kernel_task_arc: Arc<dyn TaskTrait> = Arc::new(kernel_task_obj);

    TASK_QUEUE.lock().push_back(Some(Task {
        task_inner: kernel_task_arc.clone(),
        task_future: Box::pin(future),
    }));
    TASK_HASH_MAP
        .lock()
        .insert(kernel_task_arc.get_task_id(), kernel_task_arc.clone());
}


pub fn run_task()
{
    GLOBLE_EXECUTOR.lock().run();
}
