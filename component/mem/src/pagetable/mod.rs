<<<<<<< HEAD
//! 页表模块
//!
//! 按不同架构导出对应实现。

=======
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
#[allow(unused_imports)]
/// 导出riscv64架构页表实现。
pub use riscv64::*;
#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
#[allow(unused_imports)]
/// 导出aarch64架构页表实现。
pub use aarch64::*;
#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
/// 导出x86_64架构页表实现。
pub use x86_64::*;
#[cfg(target_arch = "loongarch64")]
mod loongarch64;
#[cfg(target_arch = "loongarch64")]
#[allow(unused_imports)]
/// 导出loongarch64架构页表实现。
pub use loongarch64::*;



