use core::fmt::Write;
#[allow(deprecated)]
use sbi_rt::legacy::console_putchar;

pub fn putch(char: u8) {
    #[allow(deprecated)]
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
