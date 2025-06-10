//! RISC-V 64 QEMU-specific kernel configuration

#![allow(dead_code)]

pub mod plat {
    pub const PAGE_SIZE: usize = 0x1000;
    pub const VIRT_ADDR_START: usize = 0xffff_ffc0_0000_0000;
    pub const HEAP_SIZE: usize = 0x10_0000;
    pub const STACK_SIZE: usize = 0x10_0000;
    pub const FRAME_SIZE: usize = 512 * 1024 * 1024;

    /// 用户态动态链接用户程序的偏移
    pub const USER_DYN_ADDR: usize = 0x20000000;

    /// 用户态栈顶
    pub const USER_STACK_TOP: usize = 0x8000_0000;

    /// 用户栈初始大小
    pub const USER_STACK_INIT_SIZE: usize = 0x20000;
}
