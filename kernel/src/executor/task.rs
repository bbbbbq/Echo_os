use crate::executor::id_alloc::{alloc_tid, dealloc_tid, TaskId};
use mem::pagetable::get_boot_page_table;
use alloc::sync::Arc;
use alloc::boxed::Box;
use core::pin::Pin;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Kernel,
    User,
}

/// A task that can be executed asynchronously
pub trait AsyncTask: Send + Sync {
    /// Get the id of the task
    fn get_task_id(&self) -> TaskId;
    /// Run before the kernel
    fn before_run(&self);
    /// Get task type.
    fn get_task_type(&self) -> TaskType;
    /// Exit a task with exit code.
    fn exit(&self, exit_code: usize);
    /// Check if the task was exited successfully
    fn exit_code(&self) -> Option<usize>;
}

pub type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub struct AsyncTaskItem {
    pub future: PinedFuture,
    pub task: Arc<dyn AsyncTask>,
}

pub struct KernelTask {
    id: TaskId,
}

impl KernelTask {
    pub fn new() -> Self {
        Self { id: alloc_tid() }
    }
}

impl Drop for KernelTask {
    fn drop(&mut self) {
        dealloc_tid(self.id);
    }
}

impl AsyncTask for KernelTask {
    fn get_task_id(&self) -> TaskId {
        self.id
    }

    fn before_run(&self) {
        get_boot_page_table().change_pagetable();
    }

    fn get_task_type(&self) -> TaskType {
        TaskType::Kernel
    }

    fn exit(&self, _exit_code: usize) {
        unreachable!("can't exit kernel task")
    }

    fn exit_code(&self) -> Option<usize> {
        unreachable!("can't exit kernel task")
    }
}
