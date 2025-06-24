use crate::executor::id_alloc::{alloc_tid, dealloc_tid, TaskId};
use mem::pagetable::change_boot_pagetable;
use alloc::sync::Arc;
use alloc::boxed::Box;
use core::pin::Pin;
use downcast_rs::{impl_downcast, DowncastSync};
use core::fmt::Debug;
use arch::flush_tlb;
///
/// 任务 trait 及相关类型定义。
///
/// 提供异步任务抽象、任务类型、任务包装、内核任务实现等。
/// A task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// 内核任务。
    Kernel,
    /// 用户任务。
    User,
}

/// 可异步执行的任务 trait。
pub trait AsyncTask: DowncastSync + Send + Sync + Debug {
    /// 获取任务 ID。
    fn get_task_id(&self) -> TaskId;
    /// 运行前的准备操作。
    fn before_run(&self);
    /// 获取任务类型。
    fn get_task_type(&self) -> TaskType;
    /// 以指定退出码退出任务。
    fn exit(&self, exit_code: usize);
    /// 检查任务是否已退出。
    fn exit_code(&self) -> Option<usize>;
}


pub type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
use bitflags::bitflags;

bitflags! {
    /// 任务克隆标志。
    #[derive(Debug)]
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
/// 被调度的异步任务项。
pub struct AsyncTaskItem {
    pub future: PinedFuture,
    pub task: Arc<dyn AsyncTask>,
}

impl AsyncTaskItem {
    /// 创建新的异步任务项。
    pub fn new(task: Arc<dyn AsyncTask>, future: PinedFuture) -> Self {
        Self { task, future }
    }
}

impl core::fmt::Debug for AsyncTaskItem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncTaskItem")
            .field("task", &self.task)
            .field("future", &"<Opaque Future>")
            .finish()
    }
}

/// 内核任务实现。
#[derive(Debug)]
pub struct KernelTask {
    id: TaskId,
}

impl KernelTask {
    /// 创建新的内核任务。
    pub fn new() -> Self {
        Self { id: alloc_tid() }
    }
}

impl Drop for KernelTask {
    /// 内核任务析构时自动回收任务 ID。
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
        flush_tlb();
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
