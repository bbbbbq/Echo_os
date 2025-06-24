//!
//! 信号队列与信号集合管理模块。
//!
//! 提供信号队列的添加、查询、移除、掩码等操作。

use crate::signal::{self, SigProcMask};
use crate::signal::flages::SignalFlags;

/// 实时信号数量常量。
pub const REAL_TIME_SIGNAL_NUM: usize = 33;

/// 信号队列结构。
#[derive(Debug, Clone)]
pub struct SignalList {
    /// 信号集合位图
    pub signal: usize,
}

impl SignalList {
    /// 创建新的空信号队列。
    pub fn new() -> Self {
        Self { signal: 0 }
    }

    /// 添加信号到队列。
    pub fn add_signal(&mut self, signal: SignalFlags) {
        self.signal |= signal.bits() as usize;
    }

    /// 检查队列中是否有信号。
    pub fn has_signal(&self) -> bool {
        self.signal != 0
    }

    /// 尝试获取队列中的一个信号。
    pub fn try_get_signal(&self) -> Option<SignalFlags> {
        for i in 0..64 {
            if self.signal & (1 << i) != 0 {
                return Some(SignalFlags::from_bits_truncate(1 << i));
            }
        }
        None
    }

    /// 从队列中移除指定信号。
    pub fn remove_signal(&mut self, signal: SignalFlags) {
        self.signal &= !signal.bits() as usize;
    }

    /// 检查队列中是否包含指定信号。
    pub fn has_sig(&self, signal: SignalFlags) -> bool {
        self.signal & signal.bits() as usize != 0
    }

    /// 根据掩码生成新的信号队列。
    pub fn mask(&self, mask: SigProcMask) -> SignalList {
        SignalList {
            signal: !mask.mask & self.signal,
        }
    }
}
