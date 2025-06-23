//! RISC-V 64 QEMU专用内核配置模块
//!
//! 提供RISC-V 64 QEMU环境下的内存、地址、堆栈等常量。

#![allow(dead_code)]

/// 平台相关常量定义。
pub mod plat {
    /// 页面大小（字节）
    pub const PAGE_SIZE: usize = 0x1000;
    /// 虚拟地址起始
    pub const VIRT_ADDR_START: usize = 0xffff_ffc0_0000_0000;
    /// 内核堆大小
    pub const HEAP_SIZE: usize = 0x10_0000;
    /// 内核栈大小
    pub const STACK_SIZE: usize = 0x10_0000;
    /// 物理帧大小
    pub const FRAME_SIZE: usize = 512 * 1024 * 1024;
    
    /// 用户态动态链接用户程序的偏移
    pub const USER_DYN_ADDR: usize = 0x20000000;

    /// 用户态栈顶
    pub const USER_STACK_TOP: usize = 0x8000_0000;

    /// 用户栈初始大小
    pub const USER_STACK_INIT_SIZE: usize = 0x20000;
}
