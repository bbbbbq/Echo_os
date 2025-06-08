// component/arch/src/loongarch64.rs

pub unsafe fn os_shut_down() -> ! {
    loop {
        core::arch::asm!("nop");
    }
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
pub struct FpState {
}

impl FpState {
    pub fn new() -> Self {
        Self {
            ..Self::default()
        }
    }

    pub unsafe fn save(&mut self) {
        todo!("Implement FpState::save for LoongArch64");
    }

    pub unsafe fn restore(&self) {
        todo!("Implement FpState::restore for LoongArch64");
    }
}
