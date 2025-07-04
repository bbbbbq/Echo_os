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
    warn!("Interrupt: {:?}, PC: 0x{:x}", trap_type, ctx.sepc);
    match trap_type {
        TrapType::SysCall => {
            warn!("System call from PC: 0x{:x}", ctx.sepc);
        }
        TrapType::Timer => {
            warn!("Timer interrupt received at PC: 0x{:x}", ctx.sepc);
        }
        TrapType::SupervisorExternal => {
            warn!("Supervisor external interrupt received at PC: 0x{:x}", ctx.sepc);
        }
        // 如果是异常那就panic
        TrapType::Breakpoint => {
            panic!("Breakpoint exception at PC: 0x{:x}", ctx.sepc);
        }
        TrapType::StorePageFault(addr) => {
              
            panic!("Store page fault at address 0x{:x}, PC: 0x{:x}, trap frame: {:#x?}", addr, ctx.sepc, ctx);
        }
        TrapType::LoadPageFault(addr) => {
              
            panic!("Load page fault at address 0x{:x}, PC: 0x{:x}, trap frame: {:#x?}", addr, ctx.sepc, ctx);
        }
        TrapType::InstructionPageFault(addr) => {
              
            panic!("Instruction page fault at address 0x{:x}, PC: 0x{:x}, trap frame: {:#x?}", addr, ctx.sepc, ctx);
        }
        TrapType::IllegalInstruction(inst) => {
              
            panic!("Illegal instruction: 0x{:x} at PC: 0x{:x}, trap frame: {:#x?}", inst, ctx.sepc, ctx);
        }
        TrapType::Unknown => {
            panic!("Unknown trap type at PC: 0x{:x}, trap frame: {:#x?}", ctx.sepc, ctx);
        }
    }
}
