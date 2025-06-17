use console::println;
use core::arch::asm;

#[inline(never)]
pub fn backtrace() {
    let mut fp: usize;
    unsafe {
        asm!("mv {}, s0", out(reg) fp);
    }

    println!("--- backtrace ---");
    while fp != 0 {
        // Sanity check for frame pointer
        // It should be aligned and not in the first page.
        if fp % 8 != 0 || fp < 4096 {
            break;
        }

        let prev_fp = unsafe { *(fp as *const usize) };
        let ra = unsafe { *((fp - 8) as *const usize) };

        if ra == 0 {
            break;
        }
        println!("0x{:x}", ra);

        // Frame pointer should be monotonically increasing (stack grows down).
        if prev_fp <= fp {
            break;
        }
        fp = prev_fp;
    }
    println!("--- backtrace end ---");
}
