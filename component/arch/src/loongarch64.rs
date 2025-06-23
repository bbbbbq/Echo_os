// component/arch/src/loongarch64.rs

//! LoongArch64 架构相关实现。

/// 关闭操作系统（死循环）。
///
/// # Safety
/// 仅在需要关闭系统时调用。
pub unsafe fn os_shut_down() -> ! {
    loop {
        core::arch::asm!("nop");
    }
}

/// 浮点状态保存结构体。
#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct FpState {}

impl FpState {
    /// 创建新的浮点状态。
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    /// 保存浮点状态。
    ///
    /// # Safety
    /// 需要保证调用环境正确。
    pub unsafe fn save(&mut self) {
        todo!("Implement FpState::save for LoongArch64");
    }

    /// 恢复浮点状态。
    ///
    /// # Safety
    /// 需要保证调用环境正确。
    pub unsafe fn restore(&self) {
        todo!("Implement FpState::restore for LoongArch64");
    }
}
