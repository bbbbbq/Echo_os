use uint_allocator::create_uint_allocator;
use spin::Mutex;
use lazy_static::*;


create_uint_allocator!(TaskIdAllocator,0,0x1000);




/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TaskId(pub usize);

impl TaskId
{
    pub fn new()->Self
    {
        TaskId(TaskIdAllocator.lock().alloc().unwrap())
    }
}


pub fn alloc_tid() -> TaskId {
    TaskId::new()
}

pub fn dealloc_tid(id: TaskId) {
    TaskIdAllocator.lock().dealloc(id.0);
}


