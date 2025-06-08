use core::task::{Context, Poll};

use alloc::{collections::VecDeque, vec::Vec};
use hashbrown::HashMap;
use spin::Mutex;

use crate::{
    id::TaskId,
    task_def::{Task, TaskTrait},
    waker::Waker,
};

use alloc::sync::Arc;
use lazy_static::*;

lazy_static! {
    pub static ref TASK_QUEUE: Mutex<VecDeque<Option<Task>>> = Mutex::new(VecDeque::new());
}

lazy_static! {
    pub static ref TASK_HASH_MAP: Mutex<HashMap<TaskId, Arc<dyn TaskTrait>>> =
        Mutex::new(HashMap::new());
}


lazy_static!
{
    pub static ref GLOBLE_EXECUTOR:Executor = Executor::new();
}

pub fn get_task_by_id(id: TaskId) -> Option<Arc<dyn TaskTrait>> // Return type changed
{
    let task_queue = TASK_QUEUE.lock();
    for task_option in task_queue.iter()
    // Iterates over &Option<Task>
    {
        if let Some(task_ref) = task_option
        // task_ref is &Task
        {
            if task_ref.task_inner.get_task_id() == id {
                return Some(task_ref.task_inner.clone()); // Clone the Arc<dyn TaskTrait> from task_inner
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
        Self {
            cur_task: Vec::new(),
        }
    }

    pub fn run_task(&mut self) {
        loop {
            self.run();
        }
    }

    pub fn run(&mut self) {
        if let Some(Some(owned_task)) = TASK_QUEUE.lock().pop_front() {
            let task_inner = owned_task.task_inner; // Moves Arc<dyn TaskTrait>
            let mut task_future = owned_task.task_future; // Moves PinedFuture, made mutable
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


pub fn get_cur_task()->Option<Arc<dyn TaskTrait>>
{
    GLOBLE_EXECUTOR.cur_task[0].lock().clone()    
}

pub fn spawn(task: Task) {
    TASK_HASH_MAP.lock().insert(task.task_inner.get_task_id(), task.task_inner.clone());
    TASK_QUEUE.lock().push_back(Some(task)); // Push the Task directly
}


