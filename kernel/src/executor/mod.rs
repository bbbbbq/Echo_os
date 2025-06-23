use alloc::{fmt::format, sync::Arc};
use config::target::plat::VIRT_ADDR_START;
use mem::{maptrace::MapMemTrace, memregion::MemRegionType};

use log::{debug, error, info, warn};
use memory_addr::{VirtAddr, align_down};
use trap::{
    trap::{TrapFrame, TrapType},
    trapframe::TrapFrameArgs,
};

use crate::{
    executor::{
        executor::{get_cur_task, get_cur_usr_task},
        thread::UserTask,
    },
    signal::flages::SignalFlags,
    user_handler::{handler::UserHandler, syscall},
};
use arch::flush_tlb;
use config::target::plat::PAGE_SIZE;
use core::ptr::copy_nonoverlapping;
use frame::alloc_frame;
use page_table_multiarch::{MappingFlags, PageSize};
pub mod error;
pub mod executor;
pub mod id_alloc;
pub mod initproc;
pub mod ops;
pub mod sync;
pub mod task;
pub mod thread;

use alloc::format;
/// Architecture-specific interrupt handler.
fn fmt_trap(trap: &TrapType) -> alloc::string::String {
    use alloc::string::ToString;
    match trap {
        TrapType::StorePageFault(addr) => format!("StorePageFault({:#x})", addr),
        TrapType::LoadPageFault(addr) => format!("LoadPageFault({:#x})", addr),
        TrapType::InstructionPageFault(addr) => format!("InstructionPageFault({:#x})", addr),
        TrapType::Breakpoint => "Breakpoint".to_string(),
        TrapType::SysCall => "SysCall".to_string(),
        TrapType::Timer => "Timer".to_string(),
        TrapType::SupervisorExternal => "SupervisorExternal".to_string(),
        other => format!("{:?}", other),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn _interrupt_for_arch(
    ctx: &mut TrapFrame,
    trap_type: TrapType,
    _: usize,
) {
    if let Some(task) = get_cur_task().unwrap().downcast_arc::<UserTask>().ok() {
        warn!(
            "Interrupt received: {} pc: {:#x} task_id: {:?}",
            fmt_trap(&trap_type),
            ctx.sepc,
            task.task_id
        );
        match trap_type {
            TrapType::StorePageFault(addr)
            | TrapType::InstructionPageFault(addr)
            | TrapType::LoadPageFault(addr) => {
                if addr > VIRT_ADDR_START {
                    panic!(
                        "kernel page error: {:#x} sepc: {:#x} task_id: {:?}",
                        addr,
                        ctx[TrapFrameArgs::SEPC],
                        task.task_id
                    );
                }
                if task.pcb.is_locked() {
                    unsafe {
                        task.pcb.force_unlock();
                    }
                }
                user_cow_int(task, ctx, addr.into());
            }
            // TrapType::SysCall => {
            //     warn!(
            //         "System call interrupt from PC: 0x{:x} task_id: {:?}",
            //         ctx.sepc,
            //         task.task_id
            //     );

            // }
            TrapType::Timer => {
                warn!(
                    "Timer interrupt received at PC: 0x{:x} task_id: {:?}",
                    ctx.sepc, task.task_id
                );
            }
            TrapType::SupervisorExternal => {
                warn!(
                    "Supervisor external interrupt received at PC: 0x{:x} task_id: {:?}",
                    ctx.sepc, task.task_id
                );
            }
            TrapType::Breakpoint => {
                panic!(
                    "Breakpoint exception at PC: 0x{:x} task_id: {:?}",
                    ctx.sepc, task.task_id
                );
            }
            TrapType::IllegalInstruction(inst) => {
                panic!(
                    "Illegal instruction: 0x{:x} at PC: 0x{:x}, trap frame: {:?} task_id: {:?}",
                    inst, ctx.sepc, ctx, task.task_id
                );
            }
            TrapType::Unknown => {
                panic!(
                    "Unknown trap type at PC: 0x{:x}, trap frame: {:?} task_id: {:?}",
                    ctx.sepc, ctx, task.task_id
                );
            }
            _ => {}
        }
    } else {
        warn!(
            "Interrupt received: {} pc: {:#x}, but no current task found",
            fmt_trap(&trap_type),
            ctx.sepc,
        );
        panic!("No current task during trap handling: {:#x?}", trap_type);
    }
}

/// Copy on write.
/// call this function when trigger store/instruction page fault.
/// copy page or remap page.
pub fn user_cow_int(task: Arc<UserTask>, cx_ref: &mut TrapFrame, vaddr: VirtAddr) {
    // for region in task.pcb.lock().mem_set.regions.iter() {
    //     for map_trace in region.map_traces.iter() {
    //         let strong_count = Arc::strong_count(&map_trace.frame);
    //         info!("map_trace strong count: {}", strong_count);
    //     }
    // }
    warn!(
        "store/instruction page fault @ {:#x} vaddr: {:?} paddr: {:?} task_id: {:?}",
        cx_ref[TrapFrameArgs::SEPC],
        vaddr,
        task.page_table.lock().translate(vaddr),
        task.task_id
    );
    let mut pcb = task.pcb.lock();
    let floor_va = VirtAddr::from_usize(align_down(vaddr.into(), PAGE_SIZE));
    let area = pcb
        .mem_set
        .regions
        .iter_mut()
        .find(|x| x.map_traces.iter().any(|trace| trace.vaddr == floor_va));

    if let Some(area) = area {
        let finded = area.map_traces.iter_mut().find(|x| x.vaddr == floor_va);
        let mut need_new_mapping = false;
        let ppn = match finded {
            Some(map_track) => {
                if area.region_type == MemRegionType::SHARED {
                    error!(
                        "shared page fault @ {:#x} vaddr: {:?} paddr: {:?} task_id: {:?}",
                        cx_ref[TrapFrameArgs::SEPC],
                        vaddr,
                        task.page_table.lock().translate(vaddr),
                        task.task_id
                    );
                    task.tcb.write().signal.add_signal(SignalFlags::SIGSEGV);
                    return;
                }

                debug!("strong count: {}", Arc::strong_count(&map_track.frame));
                if Arc::strong_count(&map_track.frame) > 1 {
                    let src_arc = map_track.frame.clone();
                    let src_paddr = src_arc.paddr.as_usize();
                    let src_ptr = (src_paddr | VIRT_ADDR_START) as *const u8;
                    let dst = alloc_frame().expect("can't alloc @ user page fault");
                    let dst_paddr = dst.paddr.as_usize();
                    let dst_ptr = (dst_paddr | VIRT_ADDR_START) as *mut u8;
                    unsafe {
                        copy_nonoverlapping(src_ptr, dst_ptr, PAGE_SIZE);
                    }
                    map_track.frame = Arc::new(dst);
                    need_new_mapping = true;
                }
                map_track.frame.paddr
            }
            None => {
                let new_frame = Arc::new(alloc_frame().expect("can't alloc frame in cow_fork_int"));
                let mtrace = MapMemTrace::new(
                    floor_va,
                    new_frame.clone(),
                    MappingFlags::USER
                        | MappingFlags::READ
                        | MappingFlags::WRITE
                        | MappingFlags::EXECUTE,
                );
                area.map_traces.push(mtrace.clone());
                need_new_mapping = true;
                mtrace.frame.paddr
            }
        };

        drop(pcb);
        {
            let mut pt = task.page_table.lock();
            if need_new_mapping {
                // 先尝试 remap，若 remap 失败（已存在条目不可覆盖）则执行 unmap + map
                match pt.page_table.remap(
                    floor_va,
                    ppn,
                    MappingFlags::USER
                        | MappingFlags::READ
                        | MappingFlags::WRITE
                        | MappingFlags::EXECUTE,
                ) {
                    Ok(_) => {
                        debug!("remap ok: {:#x} -> {:#x}", floor_va, ppn);
                    }
                    Err(_) => {
                        // 取消旧映射后重新建立映射
                        let _ = pt.page_table.unmap(floor_va);
                        let _ = pt.page_table.map(
                            floor_va,
                            ppn,
                            PageSize::Size4K,
                            MappingFlags::USER
                                | MappingFlags::READ
                                | MappingFlags::WRITE
                                | MappingFlags::EXECUTE,
                        );
                        debug!("new mapping: {:#x} -> {:#x}", floor_va, ppn);
                    }
                }
            } else {
                // 仅需提升权限即可
                let _ = pt.page_table.protect_region(
                    floor_va,
                    PAGE_SIZE,
                    MappingFlags::USER
                        | MappingFlags::READ
                        | MappingFlags::WRITE
                        | MappingFlags::EXECUTE,
                    true,
                );
            }
            // 映射或权限修改后，需要刷新TLB，否则可能再次触发同一地址的缺页异常
            flush_tlb();
        }
    } else {
        task.tcb.write().signal.add_signal(SignalFlags::SIGSEGV);
    }
}
