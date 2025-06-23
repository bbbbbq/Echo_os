//!
//! TrapFrame 模块：定义各架构下 TrapFrame 结构体及通用参数枚举。
//!
//! 提供跨架构的 TrapFrameArgs 枚举和 TrapFrame 大小常量。

use core::mem::size_of;


#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
#[allow(unused_imports)]
pub use riscv64::*;
#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
#[allow(unused_imports)]
pub use aarch64::*;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
pub use x86_64::*;
#[cfg(target_arch = "loongarch64")]
mod loongarch64;
#[cfg(target_arch = "loongarch64")]
#[allow(unused_imports)]
pub use loongarch64::*;

/// TrapFrame 参数枚举。
///
/// 可用于通过 Index/IndexMut trait 访问 TrapFrame 各寄存器字段。
#[derive(Debug)]
pub enum TrapFrameArgs {
    /// 异常程序计数器（PC/ELR/ERA）
    SEPC,
    /// 返回地址（RA）
    RA,
    /// 栈指针（SP）
    SP,
    /// 返回值（RET）
    RET,
    /// 参数 0
    ARG0,
    /// 参数 1
    ARG1,
    /// 参数 2
    ARG2,
    /// 参数 3
    ARG3,
    /// 参数 4
    ARG4,
    /// 参数 5
    ARG5,
    /// 线程本地存储（TLS）
    TLS,
    /// 系统调用号
    SYSCALL,
}

/// TrapFrame 结构体的字节大小。
///
/// 用于分配 TrapFrame 所需的内存空间。
pub const TRAPFRAME_SIZE: usize = size_of::<TrapFrame>();
