#![macro_use]
use uint_allocator::create_uint_allocator;

create_uint_allocator!(TASKID_ALLOCATOR, 0, 0x1000);
create_uint_allocator!(PROCID_ALLOCATOR, 0, 0x1000);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcId(pub usize);

impl core::fmt::Debug for ProcId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ProcId({})", self.0)
    }
}

impl core::fmt::Debug for TaskId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TaskId({})", self.0)
    }
}

impl TaskId {
    pub fn new() -> Self {
        Self(TASKID_ALLOCATOR.lock().alloc().unwrap())
    }

    pub fn destroy(&self) {
        TASKID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl ProcId {
    pub fn new() -> Self {
        Self(PROCID_ALLOCATOR.lock().alloc().unwrap())
    }

    pub fn destroy(&self) {
        PROCID_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub fn alloc_task_id() -> TaskId {
    TaskId(TASKID_ALLOCATOR.lock().alloc().unwrap())
}

pub fn alloc_proc_id() -> ProcId {
    ProcId(PROCID_ALLOCATOR.lock().alloc().unwrap())
}
