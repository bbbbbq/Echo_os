use uint_allocator::{create_uint_allocator, lazy_static};
use lazy_static::*;
use spin::Mutex;


create_uint_allocator!(PID_ALLOCATOR, 0, 1024);
create_uint_allocator!(PGID_ALLOCATOR, 0, 1024);
create_uint_allocator!(TID_ALLOCATOR, 0, 1024);



pub struct Pid(usize);
pub struct Pgid(usize);
pub struct Tid(usize);

impl Pid {
    pub fn get(&self) -> usize {
        self.0
    }

    pub fn new() -> Self
    {
        let id = PID_ALLOCATOR.lock().alloc().unwrap();
        Pid(id)
    }

    pub fn deploy_id(&self)
    {
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl Pgid {
    pub fn get(&self) -> usize {
        self.0
    }

    pub fn new() -> Self
    {
        let id = PGID_ALLOCATOR.lock().alloc().unwrap();
        Pgid(id)
    }

    pub fn deploy_id(&self)
    {
        PGID_ALLOCATOR.lock().dealloc(self.0);
    }
}

impl Tid {
    pub fn get(&self) -> usize {
        self.0
    }

    pub fn new() -> Self
    {
        let id = TID_ALLOCATOR.lock().alloc().unwrap();
        Tid(id)
    }

    pub fn deploy_id(&self)
    {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}
