//! 进程时间统计(TMS)结构体定义
//!
//! 用于times等系统调用，保存进程及其子进程的用户/系统时间。

/// TMS 结构体，保存进程时间信息。
#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct TMS {
    /// 用户态CPU时间
    pub utime: u64,
    /// 内核态CPU时间
    pub stime: u64,
    /// 已终止的子进程用户态CPU时间
    pub cutime: u64,
    /// 已终止的子进程内核态CPU时间
    pub cstime: u64,
}

impl core::fmt::Display for TMS {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "TMS {{ utime: {}, stime: {}, cutime: {}, cstime: {} }}",
            self.utime, self.stime, self.cutime, self.cstime
        )
    }
}