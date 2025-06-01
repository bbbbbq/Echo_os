//! RISC-V 64 QEMU-specific kernel configuration

#![allow(dead_code)]


pub mod plat
{
    pub const PAGE_SIZE:usize = 0x1000;
    pub const VIRT_ADDR_START: usize = 0xffff_ffc0_0000_0000;
    pub const HEAP_SIZE:usize = 0x10_0000;
    pub const STACK_SIZE:usize = 0x10_0000;
    pub const FRAME_SIZE:usize = 512 * 1024 * 1024;
}