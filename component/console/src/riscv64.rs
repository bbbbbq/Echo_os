use core::fmt::Write;
#[allow(deprecated)]
use sbi_rt::legacy::console_putchar;

//! RISC-V 64 架构下的控制台输出实现。
//!
//! 提供字符输出和格式化输出的支持。

/// 输出单个字符到控制台。
///
/// # 参数
/// * `char` - 要输出的字符（u8）。
pub fn putch(char: u8) {
    #[allow(deprecated)]
    console_putchar(char as usize);
}

/// 用于格式化输出到控制台的调试写入器。
pub struct DebugWriter;
impl Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            putch(c);
        }
        Ok(())
    }
}
