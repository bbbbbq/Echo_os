use uint_allocator::create_uint_allocator;

//!
//! 任务 ID 分配模块。
//!
//! 提供 TaskId 分配、回收与唯一性保证。

create_uint_allocator!(TaskIdAllocator,0,0x1000);

/// 任务唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TaskId(pub usize);

impl TaskId
{
    /// 分配新的任务 ID。
    pub fn new()->Self
    {
        TaskId(TaskIdAllocator.lock().alloc().unwrap())
    }
}

/// 分配新的任务 ID。
pub fn alloc_tid() -> TaskId {
    TaskId::new()
}

/// 回收任务 ID。
pub fn dealloc_tid(id: TaskId) {
    TaskIdAllocator.lock().dealloc(id.0);
}


