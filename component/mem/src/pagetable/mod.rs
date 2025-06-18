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



