//! Trapframe module.
//!
//!

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

/// Trap Frame Arg Type
///
/// Using this by Index and IndexMut trait bound on TrapFrame
#[derive(Debug)]
pub enum TrapFrameArgs {
    SEPC,
    RA,
    SP,
    RET,
    ARG0,
    ARG1,
    ARG2,
    ARG3,
    ARG4,
    ARG5,
    TLS,
    SYSCALL,
}

/// The size of the [TrapFrame]
pub const TRAPFRAME_SIZE: usize = size_of::<TrapFrame>();
