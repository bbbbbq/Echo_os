//!
//! 信号模块：信号掩码、信号操作方式等定义。
//!
//! 提供信号相关的基础类型和操作。
pub mod list;
pub mod flages;

/// 信号掩码。
#[derive(Debug, Clone, Copy)]
pub struct SigProcMask {
    /// 掩码位图
    pub mask: usize,
}

/// 信号掩码操作方式。
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SigMaskHow {
    /// 阻塞信号
    Block,
    /// 解除阻塞
    Unblock,
    /// 设置掩码
    Setmask,
}

impl SigMaskHow {
    /// 从 usize 转换为 SigMaskHow。
    pub fn from_usize(how: usize) -> Option<Self> {
        match how {
            0 => Some(SigMaskHow::Block),
            1 => Some(SigMaskHow::Unblock),
            2 => Some(SigMaskHow::Setmask),
            _ => None,
        }
    }
}

impl SigProcMask {
    /// 创建新的空信号掩码。
    pub fn new() -> Self {
        Self { mask: 0 }
    }

    /// 根据操作方式处理掩码。
    pub fn handle(&mut self, how: SigMaskHow, mask: &Self) {
        self.mask = match how {
            SigMaskHow::Block => self.mask | mask.mask,
            SigMaskHow::Unblock => self.mask & (!mask.mask),
            SigMaskHow::Setmask => mask.mask,
        }
    }

    /// 检查指定信号是否被掩码。
    pub fn masked(&self, signum: usize) -> bool {
        (self.mask >> signum) & 1 == 0
    }
}