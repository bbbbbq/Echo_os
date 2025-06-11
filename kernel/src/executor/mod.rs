use trap::trap::{TrapFrame, TrapType};
use log::warn;

pub mod executor;
pub mod task;
pub mod thread;
pub mod id_alloc;


#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn _interrupt_for_arch(ctx: &mut TrapFrame, trap_type: TrapType, _: usize) {
    warn!("Interrupt: {:?}", trap_type);
    match trap_type {
        TrapType::SysCall => {
            ctx.sepc += 4;
            warn!("Syscall not implemented");
        }
        _ => {
            panic!("Unhandled trap: {:?}", trap_type);
        }
    }
}
