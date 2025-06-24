//! 用户空间缓冲区（UserBuf）模块。
//!
//! 该模块提供了 `UserBuf` 结构体，用于在内核与用户空间之间安全地读写数据。
//! 主要用于系统调用参数、用户指针的封装与操作，支持字符串读取、写入、切片等常用功能。
//! 所有操作均需注意指针的有效性与内存安全。

/// 用户空间缓冲区指针封装。
///
/// `UserBuf<T>` 封装了一个指向用户空间的裸指针，
/// 提供了安全的接口用于读取、写入、偏移、切片等操作。
/// 适用于内核与用户空间的数据交互场景。
///
/// # 类型参数
/// - `T`: 指针指向的数据类型。
///
/// # 安全性
/// - 该结构体本身不保证指针的有效性，使用前需确保指针合法。
/// - 部分方法为 `unsafe` 操作，需谨慎使用。
#[derive(Debug, Clone, Copy)]
pub struct UserBuf<T> {
    /// 指向用户空间的裸指针。
    pub ptr: *mut T
}

/// 为 `UserBuf` 实现 `Display`，便于调试输出。
impl<T> core::fmt::Display for UserBuf<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UserBuf({:p})", self.ptr)
    }
}

/// 支持将 `UserBuf` 转换为 usize，便于地址传递。
impl<T> Into<usize> for UserBuf<T> {
    fn into(self) -> usize {
        self.ptr as usize
    }
}

/// `UserBuf` 可以安全地在多线程间传递。
unsafe impl<T> Send for UserBuf<T> {}
unsafe impl<T> Sync for UserBuf<T> {}

use crate::alloc::string::String;
use crate::alloc::vec::Vec;

/// 用户空间路径字符串的最大长度。
const MAX_PATH: usize = 256;

impl<T: Copy> UserBuf<T> {
    /// 从用户空间读取一个 T 类型的值。
    ///
    /// # 返回值
    /// 返回指针处的 T 类型数据。
    ///
    /// # 安全性
    /// - 需保证指针有效且可读。
    pub fn read(&self) -> T {
        unsafe { self.ptr.read() }
    }
}

impl<T> UserBuf<T> {
    /// 从用户空间读取以 0 结尾的字符串（C 字符串），并转换为 Rust 的 `String`。
    ///
    /// 最多读取 `MAX_PATH` 字节，遇到 0 字节提前结束。
    ///
    /// # 返回值
    /// 返回读取到的字符串，若解码失败则返回空字符串。
    ///
    /// # 安全性
    /// - 需保证指针有效且指向合法的用户空间内存。
    pub fn read_string(&self) -> String {
        let mut buffer: Vec<u8> = Vec::new();
        let base_ptr = self.ptr as *const u8;
        for i in 0..MAX_PATH {
            let char_ptr = unsafe { base_ptr.add(i) };
            let char_val = unsafe { char_ptr.read_volatile() };
            if char_val == 0 {
                break;
            }
            buffer.push(char_val);
        }
        String::from_utf8(buffer).unwrap_or_default()
    }

    /// 创建一个新的 `UserBuf`。
    ///
    /// # 参数
    /// - `ptr`: 指向用户空间的裸指针。
    ///
    /// # 返回值
    /// 返回新的 `UserBuf` 实例。
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }
    
    /// 获取指针处的 C 字符串（假定长度为 64 字节），并转换为 `&str`。
    ///
    /// # 返回值
    /// 返回字符串切片，若解码失败会 panic。
    ///
    /// # 安全性
    /// - 需保证指针有效且指向合法的 UTF-8 字符串。
    pub fn get_cstr(&self) -> &str {
        unsafe {
            core::str::from_utf8(core::slice::from_raw_parts(self.ptr as *const u8, 64)).unwrap()
        }
    }

    /// 获取指针处的引用。
    ///
    /// # 返回值
    /// 返回指针处的 `&T` 引用。
    ///
    /// # 安全性
    /// - 需保证指针有效且生命周期合法。
    pub fn get_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }

    /// 向用户空间写入一个 T 类型的值。
    ///
    /// # 参数
    /// - `value`: 要写入的数据。
    ///
    /// # 安全性
    /// - 需保证指针有效且可写。
    pub fn write(&self, value: T) {
        unsafe {
            self.ptr.write_volatile(value);
        }
    }

    /// 向用户空间写入一个字节切片。
    ///
    /// # 参数
    /// - `data`: 要写入的数据切片。
    ///
    /// # 安全性
    /// - 需保证指针有效且有足够空间。
    pub fn write_slice(&self, data: &[u8]) {
        unsafe {
            let len = data.len();
            let dst_slice = core::slice::from_raw_parts_mut(self.ptr as *mut u8, len);
            dst_slice.copy_from_slice(data);
        }
    }
    
    /// 判断指针是否有效（非空）。
    ///
    /// # 返回值
    /// 若指针非空返回 true，否则返回 false。
    pub const fn is_valid(&self) -> bool {
        !self.ptr.is_null()
    }

    /// 获取偏移后的 `UserBuf`。
    ///
    /// # 参数
    /// - `count`: 偏移量（以 T 为单位）。
    ///
    /// # 返回值
    /// 返回偏移后的新 `UserBuf`。
    ///
    /// # 安全性
    /// - 需保证偏移后指针仍然有效。
    pub fn offset(&self, count: isize) -> Self {
        Self { ptr: unsafe { self.ptr.offset(count) } }
    }

    /// 获取指定长度的可变切片。
    ///
    /// # 参数
    /// - `len`: 切片长度。
    ///
    /// # 返回值
    /// 返回指向用户空间的可变切片。
    /// 若指针为空或长度为 0，则返回空切片。
    ///
    /// # 安全性
    /// - 需保证指针和长度合法。
    pub fn slice_mut_with_len(&self, len: usize) -> &mut [T] {
        if self.ptr.is_null() || len == 0 {
            unsafe { core::slice::from_raw_parts_mut(core::ptr::NonNull::dangling().as_ptr(), 0) }
        } else {
            unsafe { core::slice::from_raw_parts_mut(self.ptr, len) }
        }
    }
}
