use sbi_rt::{system_reset, NoReason, Shutdown, SystemFailure};
pub fn os_shut_down() -> ! {
    system_reset(Shutdown, NoReason);
    unreachable!()
}



