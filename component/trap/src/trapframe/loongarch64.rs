use core::ops::{Index, IndexMut};

use super::TrapFrameArgs;

///
/// LoongArch64 架构下的 TrapFrame 实现。
///
/// 提供异常/中断发生时的寄存器保存结构及相关操作。
/// TrapFrame 结构体，表示一次异常/中断发生时保存的寄存器状态。
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapFrame {
    /// 通用寄存器 x0~x31
    pub regs: [usize; 32],
    /// 异常前的模式信息（PRMD）
    pub prmd: usize,
    /// 异常返回地址（ERA）
    pub era: usize,
}

impl TrapFrame {
    /// 创建新的 TrapFrame，上下文初始化。
    #[inline]
    pub fn new() -> Self {
        Self {
            // bit 1:0 PLV
            // bit 2 PIE
            // bit 3 PWE
            prmd: (0b0111),
            ..Default::default()
        }
    }
}

impl TrapFrame {
    /// 系统调用返回时，推进 ERA。
    pub fn syscall_ok(&mut self) {
        self.era += 4;
    }

    /// 获取系统调用参数（前 6 个）。
    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [
            self.regs[4],
            self.regs[5],
            self.regs[6],
            self.regs[7],
            self.regs[8],
            self.regs[9],
        ]
    }
}

impl Index<TrapFrameArgs> for TrapFrame {
    type Output = usize;
    /// 按 TrapFrameArgs 枚举索引 TrapFrame 字段。
    fn index(&self, index: TrapFrameArgs) -> &Self::Output {
        match index {
            TrapFrameArgs::SEPC => &self.era,
            TrapFrameArgs::RA => &self.regs[1],
            TrapFrameArgs::SP => &self.regs[3],
            TrapFrameArgs::RET => &self.regs[4],
            TrapFrameArgs::ARG0 => &self.regs[4],
            TrapFrameArgs::ARG1 => &self.regs[5],
            TrapFrameArgs::ARG2 => &self.regs[6],
            TrapFrameArgs::TLS => &self.regs[2],
            TrapFrameArgs::SYSCALL => &self.regs[11],
        }
    }
}

impl IndexMut<TrapFrameArgs> for TrapFrame {
    /// 按 TrapFrameArgs 枚举可变索引 TrapFrame 字段。
    fn index_mut(&mut self, index: TrapFrameArgs) -> &mut Self::Output {
        match index {
            TrapFrameArgs::SEPC => &mut self.era,
            TrapFrameArgs::RA => &mut self.regs[1],
            TrapFrameArgs::SP => &mut self.regs[3],
            TrapFrameArgs::RET => &mut self.regs[4],
            TrapFrameArgs::ARG0 => &mut self.regs[4],
            TrapFrameArgs::ARG1 => &mut self.regs[5],
            TrapFrameArgs::ARG2 => &mut self.regs[6],
            TrapFrameArgs::TLS => &mut self.regs[2],
            TrapFrameArgs::SYSCALL => &mut self.regs[11],
        }
    }
}
