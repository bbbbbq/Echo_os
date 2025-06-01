use sbi_rt::legacy::console_putchar;
use core::fmt::Write;

pub fn putch(char:u8) {
    console_putchar(char as usize);
}

pub struct DebugWriter;
impl Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            putch(c);
        }
        Ok(())
    }
}