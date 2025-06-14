use trap::trap::{TrapFrame, TrapType};
use log::warn;

pub mod executor;
pub mod task;
pub mod thread;
pub mod id_alloc;
pub mod error;
pub mod initproc;
pub mod ops;
pub mod sync;
/// Architecture-specific interrupt handler.
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn _interrupt_for_arch(ctx: &mut TrapFrame, trap_type: TrapType, _: usize) {
    warn!("Interrupt: {:?}", trap_type);
    match trap_type {
        TrapType::SysCall => {}
        TrapType::Timer => {
            warn!("Timer interrupt received");
        }
        TrapType::SupervisorExternal => {
            warn!("Supervisor external interrupt received");
        }
        // 如果是异常那就panic
        TrapType::Breakpoint => {
            panic!("Breakpoint exception");
        }
        TrapType::StorePageFault(addr) => {
            panic!("Store page fault at address 0x{:x}", addr);
        }
        TrapType::LoadPageFault(addr) => {
            panic!("Load page fault at address 0x{:x}", addr);
        }
        TrapType::InstructionPageFault(addr) => {
            panic!("Instruction page fault at address 0x{:x}", addr);
        }
        TrapType::IllegalInstruction(inst) => {
            panic!("Illegal instruction: 0x{:x} at pc=0x{:x}", inst, ctx.sepc);
        }
        TrapType::Unknown => {
            panic!("Unknown trap type");
        }
    }
}
