#![no_std]
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

// Re-export dependencies needed by the macro
pub use lazy_static::lazy_static;

//!
//! UintAllocator 模块：无符号整数分配器。
//!
//! 提供简单的区间整数分配与回收，支持宏自动生成静态分配器。
/// 无符号整数分配器。
pub struct UintAllocator {
    start: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl UintAllocator {
    /// 创建一个新的分配器，分配区间为 [start, end)。
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            recycled: vec::Vec::new(),
        }
    }

    /// 分配一个可用整数。
    ///
    /// # 返回值
    /// 返回 Some(usize) 表示分配成功，None 表示无可用整数。
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

    /// 回收一个整数。
    ///
    /// # 参数
    /// - `uint`: 要回收的整数。
    ///
    /// # Panic
    /// 如果回收的整数超出分配区间会 panic。
    pub fn dealloc(&mut self, uint: usize) {
        if uint >= self.start || uint < self.end {
            self.recycled.push(uint);
        } else {
            panic!("uint_allocator dealloc error");
        }
    }
}

/// 创建静态 UintAllocator 分配器的宏。
///
/// # 用法
/// ```
/// create_uint_allocator!(NAME, START, END);
/// ```
#[macro_export]
macro_rules! create_uint_allocator {
    ($name:ident, $start:expr, $end:expr) => {
        lazy_static::lazy_static! {
            pub static ref $name: spin::Mutex<$crate::UintAllocator> =
                spin::Mutex::new($crate::UintAllocator::new($start, $end));
        }
    };
}
