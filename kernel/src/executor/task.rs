use crate::executor::id_alloc::{alloc_tid, dealloc_tid, TaskId};
use mem::pagetable::change_boot_pagetable;
use alloc::sync::Arc;
use alloc::boxed::Box;
use core::pin::Pin;
use downcast_rs::{impl_downcast, DowncastSync};
use core::fmt::Debug;
use arch::flush;
/// A task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Kernel,
    User,
}

/// A task that can be executed asynchronously
pub trait AsyncTask: DowncastSync + Send + Sync + Debug {
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


bitflags! {
    pub struct CloneFlags: usize {
        const CSIGNAL      = 0x000000ff;
        const VM           = 0x00000100;
        const FS           = 0x00000200;
        const FILES        = 0x00000400;
        const SIGHAND      = 0x00000800;
        const PTRACE       = 0x00002000;
        const VFORK        = 0x00004000;
        const PARENT       = 0x00008000;
        const THREAD       = 0x00010000;
        const NEWNS        = 0x00020000;
        const SYSVSEM      = 0x00040000;
        const SETTLS       = 0x00080000;
        const PARENT_SETTID = 0x00100000;
        const CHILD_CLEARTID = 0x00200000;
        const DETACHED     = 0x00400000;
        const UNTRACED     = 0x00800000;
        const CHILD_SETTID = 0x01000000;
        const NEWCGROUP    = 0x02000000;
        const NEWUTS       = 0x04000000;
        const NEWIPC       = 0x08000000;
        const NEWUSER      = 0x10000000;
        const NEWPID       = 0x20000000;
        const NEWNET       = 0x40000000;
        const IO           = 0x80000000;
    }
}
pub struct AsyncTaskItem {
    pub future: PinedFuture,
    pub task: Arc<dyn AsyncTask>,
}

impl core::fmt::Debug for AsyncTaskItem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncTaskItem")
            .field("task", &self.task)
            .field("future", &"<Opaque Future>")
            .finish()
    }
}

#[derive(Debug)]
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
        change_boot_pagetable();
        flush();
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


impl_downcast!(sync AsyncTask);
