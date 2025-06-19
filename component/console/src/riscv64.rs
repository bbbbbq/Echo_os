use core::fmt::Write;
#[allow(deprecated)]
use sbi_rt::legacy::{console_putchar, console_getchar};
use core::sync::atomic::{AtomicBool, Ordering};

static INPUT_BUSY: AtomicBool = AtomicBool::new(false);

pub fn putch(char: u8) {
    #[allow(deprecated)]
    console_putchar(char as usize);
}

pub fn getch() -> Option<u8> {
    // 使用锁避免多个任务同时读取造成冲突
    if INPUT_BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return None;
    }
    
    #[allow(deprecated)]
    let ch = console_getchar();
    
    INPUT_BUSY.store(false, Ordering::Release);
    
    if ch == 0 || ch > 255 {
        None  // 没有输入或输入无效
    } else {
        Some(ch as u8)
    }
}

/// 检查是否有输入可用，不阻塞
pub fn has_input() -> bool {
    getch().is_some()
}

/// 从控制台读取字符，如果没有输入则尝试多次检测
/// 注意：这是一个阻塞调用，会自旋等待输入
pub fn get_char_blocking() -> u8 {
    loop {
        if let Some(ch) = getch() {
            return ch;
        }
        // 短暂自旋等待
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
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
