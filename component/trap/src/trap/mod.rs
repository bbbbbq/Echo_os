//!
//! trap 模块：定义并初始化各架构下的 trap（异常/中断）处理。
//!
//! 提供 TrapType、EscapeReason 等通用类型，并根据目标架构导出 trap 相关实现。

pub use super::trapframe::TrapFrame;

/// trap 类型枚举，表示不同的异常或中断类型。
#[derive(Debug, Clone, Copy)]
pub enum TrapType {
    /// 断点异常。
    Breakpoint,
    /// 系统调用。
    SysCall,
    /// 定时器中断。
    Timer,
    /// 未知异常。
    Unknown,
    /// 外部中断（Supervisor External）。
    SupervisorExternal,
    /// 存储页错误。
    StorePageFault(usize),
    /// 加载页错误。
    LoadPageFault(usize),
    /// 指令页错误。
    InstructionPageFault(usize),
    /// 非法指令。
    IllegalInstruction(usize),
}

/// 任务逃逸原因枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeReason {
    /// 无特殊原因。
    NoReason,
    /// 外部中断。
    IRQ,
    /// 定时器中断。
    Timer,
    /// 系统调用。
    SysCall,
}

/// TrapType 到 EscapeReason 的转换实现。
impl From<TrapType> for EscapeReason {
    fn from(value: TrapType) -> Self {
        match value {
            TrapType::SysCall => EscapeReason::SysCall,
            TrapType::Timer => EscapeReason::Timer,
            TrapType::SupervisorExternal => EscapeReason::IRQ,
            _ => EscapeReason::NoReason,
        }
    }
}

/// 架构相关的 trap 处理回调。
///
/// # Safety
/// 该函数为 FFI 接口，需保证 ctx、trap_type、token 的合法性。
unsafe extern "Rust" {
    pub(crate) fn _interrupt_for_arch(ctx: &mut TrapFrame, trap_type: TrapType, token: usize);
}

#[cfg(target_arch = "riscv64")]
pub mod riscv64;
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
