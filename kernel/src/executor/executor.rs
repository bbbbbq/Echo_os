use super::task::AsyncTask;
use super::task::AsyncTaskItem;
use alloc::{sync::Arc, vec::Vec};
use arch::{get_cpu_num, get_cur_cpu_id};
use futures_lite::future::yield_now;
use log::debug;
use log::info;
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicUsize;
use spin::Mutex;
use crate::executor::error;
use crate::executor::id_alloc::TaskId;
use crate::executor::thread::UserTask;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::task::Wake;
use core::task::Context;
use core::task::Poll;
use lazy_static::*;
use alloc::boxed::Box;
use crate::executor::task::KernelTask;
/// Global task queue
pub(crate) static TASK_QUEUE: Mutex<VecDeque<AsyncTaskItem>> = Mutex::new(VecDeque::new());

lazy_static! {
    pub static ref GLOBLE_EXECUTOR: Executor = Executor::new();
    pub static ref TASK_MAP: Mutex<BTreeMap<TaskId, Arc<dyn AsyncTask>>> = Mutex::new(BTreeMap::new());
}

/// Executor
pub struct Executor {
    cores: Vec<Mutex<Option<Arc<dyn AsyncTask>>>>,
    is_inited: AtomicBool,
}

/// Waker
pub struct Waker {
    #[allow(dead_code)]
    task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {}
}

impl Executor {
    /// Create a new executor
    pub fn new() -> Self {
        let cpu_num = get_cpu_num();
        let mut cores: Vec<spin::mutex::Mutex<Option<Arc<dyn AsyncTask + 'static>>>> = Vec::with_capacity(cpu_num);
        for _ in 0..cpu_num {
            cores.push(Mutex::new(None));
        }
        Self {
            cores,
            is_inited: AtomicBool::new(true),
        }
    }

    /// Spawn a new task
    pub fn spawn(&self, task: AsyncTaskItem) {
        let task_id = task.task.get_task_id();
        TASK_MAP.lock().insert(task_id, task.task.clone());
        TASK_QUEUE.lock().push_back(task);
    }

    /// Run a ready task
    pub fn run_ready_task(&self) {
        // debug!("run_ready_task");
        assert!(
            self.is_inited.load(core::sync::atomic::Ordering::Acquire),
            "Executor not initialized"
        );

        let task = {
            let mut task_queue = TASK_QUEUE.lock();
            task_queue.pop_front()
        };
        if let Some(task) = task {
            let task_id = task.task.get_task_id();
            info!("run_ready_task poll Task {:?}", task_id);
            
            let AsyncTaskItem { task, mut future } = task;
            
            task.before_run();
            // info!("task : {:?}", task);
            let cur_cpu_id = get_cur_cpu_id();
            *self.cores[cur_cpu_id].lock() = Some(task.clone());
            let waker = Arc::new(Waker {
                task_id: task.get_task_id(),
            })
            .into();
            let mut context: Context<'_> = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    info!("run_ready_task Task {:?} completed", task_id);
                    // 任务已完成，从任务映射表中移除
                    release_task(task_id);
                }
                Poll::Pending => {
                    info!("run_ready_task Task {:?} pending, re-queue", task_id);
                    TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task });
                    yield_now();
                }
            }
        }
    }

    pub fn run(&self) {
        loop {
            static PRINT_COUNTER: AtomicUsize = AtomicUsize::new(0);
            if PRINT_COUNTER.fetch_add(1, Ordering::Relaxed) % 5000 == 0 {
                let task_ids: Vec<_> = TASK_QUEUE.lock().iter().map(|task| task.task.get_task_id()).collect();
                info!("TASK_QUEUE IDs: {:?}", task_ids);
                
                let task_map_ids: Vec<_> = TASK_MAP.lock().keys().cloned().collect();
                info!("TASK_MAP IDs: {:?}", task_map_ids);
            }
            if TASK_QUEUE.lock().is_empty() && TASK_MAP.lock().is_empty() {
                info!("No tasks remaining, shutting down.");
                arch::os_shut_down();
            }

            self.run_ready_task();
        }
    }

}

/// Add a ready task to the task queue
pub fn add_ready_task(task: AsyncTaskItem) {
    TASK_QUEUE.lock().push_back(task);
}

/// Get a task by its task ID
pub fn tid2task(tid: TaskId) -> Option<Arc<dyn AsyncTask>> {
    let task_map = TASK_MAP.lock();
    task_map.get(&tid).cloned()
}

/// Get the current user task
pub fn get_cur_usr_task() -> Option<Arc<UserTask>> {
    let executor = &GLOBLE_EXECUTOR;
    let task_option_guard = executor.cores[get_cur_cpu_id()].lock();
    task_option_guard
        .as_ref()
        .and_then(|task| task.clone().downcast_arc::<UserTask>().ok())
}

/// Release a task
pub fn release_task(task_id: TaskId) {
    error!("release task: {:?}", task_id);
    TASK_MAP.lock().remove(&task_id);
    let executor = &GLOBLE_EXECUTOR;
    for core_lock in executor.cores.iter() {
        let mut guard = core_lock.lock();
        if let Some(task) = guard.as_ref() {
            if task.get_task_id() == task_id {
                guard.take();
            }
        }
    }

    // // 移除任务队列中残留的待释放任务
    // TASK_QUEUE.lock().retain(|item: &AsyncTaskItem| item.task.get_task_id() != task_id);
}

/// Spawn a blank task

#[inline]
pub fn spawn_blank(future: impl Future<Output = ()> + Send + 'static) {
    let task: Arc<dyn AsyncTask> = Arc::new(KernelTask::new());
    TASK_QUEUE.lock().push_back(AsyncTaskItem {
        future: Box::pin(future),
        task,
    })
}

pub fn spawn(task: Arc<dyn AsyncTask>, future: impl Future<Output = ()> + Send + 'static) {
    GLOBLE_EXECUTOR.spawn(AsyncTaskItem {
        future: Box::pin(future),
        task,
    })
}

pub fn info_task_queue() {
    for task in TASK_QUEUE.lock().iter() {
        let inner = task.task.clone();
        info!("task : {:?}", inner);
    }
}


pub fn get_cur_task() -> Option<Arc<dyn AsyncTask>> {
    let executor = &GLOBLE_EXECUTOR;
    let task_option_guard = executor.cores[get_cur_cpu_id()].lock();
    task_option_guard.clone()
}