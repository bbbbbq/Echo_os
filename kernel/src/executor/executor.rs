use super::task::AsyncTask;
use super::task::AsyncTaskItem;
use alloc::{sync::Arc, vec::Vec};
use arch::{get_cpu_num, get_cur_cpu_id};
use log::info;
use core::sync::atomic::AtomicBool;
use spin::Mutex;
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

/// Global executor
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
            info!("Running task with ID: {:?}", task_id);
            
            let AsyncTaskItem { task, mut future } = task;

            // ****************** test ****************** 

            // if let Some(task_ref) = task.downcast_ref::<UserTask>() {
            //     // Task is a UserTask
            //     info!("Running UserTask with ID: {:?}", task_ref.get_task_id());
            //     let pagetable = task_ref.page_table.clone();
            //     // 测试高于0xffffffc的地址
            //     let test_addr = VirtAddr::from(0xffff_ffc0_0000_0000);
            //     if let Some(phys_addr) = pagetable.translate(test_addr) {
            //         info!("Translated 0xffff_ffc0_0000_0000 to physical address: {:?}", phys_addr);
            //     } else {
            //         error!("Failed to translate address 0xffff_ffc0_0000_0000");
            //     }
            // }

            // ****************** test_end ****************** 
            
            task.before_run();
            // info!("task : {:?}", task);
            let cur_cpu_id = get_cur_cpu_id();
            *self.cores[cur_cpu_id].lock() = Some(task.clone());
            let waker = Arc::new(Waker {
                task_id: task.get_task_id(),
            })
            .into();
            let mut context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    info!("Task {:?} completed", task_id);
                }
                Poll::Pending => TASK_QUEUE.lock().push_back(AsyncTaskItem { future, task }),
            }
        }
    }

    pub fn run(&self) {
        loop {
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
    TASK_MAP.lock().remove(&task_id);
    let executor = &GLOBLE_EXECUTOR;
    executor.cores[get_cur_cpu_id()].lock().take();
    TASK_QUEUE.lock().retain(|item| item.task.get_task_id() != task_id);    
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


pub fn info_task_queue() {
    for task in TASK_QUEUE.lock().iter() {
        let inner = task.task.clone();
        info!("task : {:?}", inner);
    }
}
