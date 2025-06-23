use core::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use riscv::register::sstatus::{self, SPP, Sstatus};

use super::TrapFrameArgs;

//!
//! RISC-V 64 架构下的 TrapFrame 实现。
//!
//! 提供异常/中断发生时的寄存器保存结构及相关操作。

#[repr(C)]
#[derive(Clone)]
/// TrapFrame 结构体，表示一次异常/中断发生时保存的寄存器状态。
pub struct TrapFrame {
    /// 32 个通用寄存器
    pub x: [usize; 32],
    /// sstatus 寄存器
    pub sstatus: Sstatus,
    /// 异常程序计数器
    pub sepc: usize,
    /// 浮点扩展寄存器
    pub fsx: [usize; 2],
}

impl Debug for TrapFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("ra", &self.x[1])
            .field("sp", &self.x[2])
            .field("gp", &self.x[3])
            .field("tp", &self.x[4])
            .field("t0", &self.x[5])
            .field("t1", &self.x[6])
            .field("t2", &self.x[7])
            .field("s0", &self.x[8])
            .field("s1", &self.x[9])
            .field("a0", &self.x[10])
            .field("a1", &self.x[11])
            .field("a2", &self.x[12])
            .field("a3", &self.x[13])
            .field("a4", &self.x[14])
            .field("a5", &self.x[15])
            .field("a6", &self.x[16])
            .field("a7", &self.x[17])
            .field("s2", &self.x[18])
            .field("s3", &self.x[19])
            .field("s4", &self.x[20])
            .field("s5", &self.x[21])
            .field("s6", &self.x[22])
            .field("s7", &self.x[23])
            .field("s8", &self.x[24])
            .field("s9", &self.x[25])
            .field("s10", &self.x[26])
            .field("s11", &self.x[27])
            .field("t3", &self.x[28])
            .field("t4", &self.x[29])
            .field("t5", &self.x[30])
            .field("t6", &self.x[31])
            .field("sstatus", &self.sstatus)
            .field("sepc", &self.sepc)
            .field("fsx", &self.fsx)
            .finish()
    }
}

impl TrapFrame {
    /// 创建新的 TrapFrame，上下文初始化。
    #[inline]
    pub fn new() -> Self {
        TrapFrame {
            x: [0usize; 32],
            sstatus: sstatus::read(),
            sepc: 0,
            fsx: [0; 2],
        }
    }

    /// 获取系统调用参数（前 6 个）。
    #[inline]
    pub fn args(&self) -> [usize; 6] {
        self.x[10..16].try_into().expect("args slice force convert")
    }

    /// 判断 TrapFrame 是否来自用户态。
    #[inline]
    pub fn from_user(&self) -> bool {
        self.sstatus.spp() == SPP::User
    }

    /// 系统调用返回时，推进 sepc。
    #[inline]
    pub fn syscall_ok(&mut self) {
        self.sepc += 4;
    }

    /// 获取栈指针。
    pub fn get_sp(&self) -> usize {
        self.x[2]
    }

    /// 设置栈指针。
    pub fn set_sp(&mut self, sp: usize)
    {
        self.x[2] = sp;
    }

    /// 设置异常程序计数器。
    pub fn set_sepc(&mut self, sepc: usize)
    {
        self.sepc = sepc;
    }

    /// 获取系统调用号。
    pub fn get_sysno(&self) -> usize
    {
        self.x[17]
    }
}

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;

    /// 按 TrapFrameArgs 枚举索引 TrapFrame 字段。
    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        match index {
            TrapFrameArgs::SEPC => &self.sepc,
            TrapFrameArgs::RA => &self.x[1],
            TrapFrameArgs::SP => &self.x[2],
            TrapFrameArgs::RET => &self.x[10],
            TrapFrameArgs::ARG0 => &self.x[11],
            TrapFrameArgs::ARG1 => &self.x[12],
            TrapFrameArgs::ARG2 => &self.x[13],
            TrapFrameArgs::ARG3 => &self.x[14],
            TrapFrameArgs::ARG4 => &self.x[15],
            TrapFrameArgs::ARG5 => &self.x[16],
            TrapFrameArgs::TLS => &self.x[4],
            TrapFrameArgs::SYSCALL => &self.x[17],
        }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    /// 按 TrapFrameArgs 枚举可变索引 TrapFrame 字段。
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        match index {
            TrapFrameArgs::SEPC => &mut self.sepc,
            TrapFrameArgs::RA => &mut self.x[1],
            TrapFrameArgs::SP => &mut self.x[2],
            TrapFrameArgs::RET => &mut self.x[10],
            TrapFrameArgs::ARG0 => &mut self.x[10],
            TrapFrameArgs::ARG1 => &mut self.x[11],
            TrapFrameArgs::ARG2 => &mut self.x[12],
            TrapFrameArgs::ARG3 => &mut self.x[13],
            TrapFrameArgs::ARG4 => &mut self.x[14],
            TrapFrameArgs::ARG5 => &mut self.x[15],
            TrapFrameArgs::TLS => &mut self.x[4],
            TrapFrameArgs::SYSCALL => &mut self.x[17],
        }
    }
}
