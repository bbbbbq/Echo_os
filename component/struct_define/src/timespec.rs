//! 时间规格(TimeSpec)结构体定义
//!
//! 用于描述秒和纳秒级时间。

/// TimeSpec 结构体，保存秒和纳秒。
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct TimeSpec {
    /// 秒数
    pub sec: usize,
    /// 纳秒数
    pub nsec: usize,
}
