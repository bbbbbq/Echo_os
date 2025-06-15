#![no_std]
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

// Re-export dependencies needed by the macro
pub use lazy_static::lazy_static;
pub struct UintAllocator {
    start: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl UintAllocator {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            recycled: vec::Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Option<usize> {
        if let Some(uint) = self.recycled.pop() {
            Some(uint)
        } else if self.start < self.end {
            let uint = self.start;
            self.start += 1;
            Some(uint)
        } else {
            None
        }
    }

    pub fn dealloc(&mut self, uint: usize) {
        if uint >= self.start || uint < self.end {
            self.recycled.push(uint);
        } else {
            panic!("uint_allocator dealloc error");
        }
    }
}

#[macro_export]
macro_rules! create_uint_allocator {
    ($name:ident, $start:expr, $end:expr) => {
        lazy_static::lazy_static! {
            pub static ref $name: spin::Mutex<$crate::UintAllocator> =
                spin::Mutex::new($crate::UintAllocator::new($start, $end));
        }
    };
}
