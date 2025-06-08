use sbi_rt::{system_reset, NoReason, Shutdown};
pub fn os_shut_down() -> ! {
    system_reset(Shutdown, NoReason);
    unreachable!()
}




#[derive(Debug, Copy, Clone, Default)]
pub struct FpState {}

impl FpState {
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn save(&mut self) {}

    pub fn restore(&self) {}
}

