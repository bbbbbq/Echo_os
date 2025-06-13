use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
use trap::trap::EscapeReason;
use trap::trapframe::{TrapFrame, TrapFrameArgs};
use trap::trap::run_user_task;
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;
use crate::user_handler::syscall::sysnum::SYS_WRITE;
use memory_addr::VirtAddr;
use log::error;
use log::info;
pub mod fs;
pub mod mem;
pub mod proc;

impl UserHandler {
    pub async fn handle_syscall(&mut self, cx_ref: &mut TrapFrame) -> UserTaskControlFlow {
        if matches!(run_user_task(cx_ref), EscapeReason::SysCall) {
            if cx_ref.get_sysno() == sysnum::SYS_SIGRETURN as _ {
                return UserTaskControlFlow::Break;
            }
            info!(
                "[task {:?}] syscall: {} at sepc: {:#x}",
                self.task.get_task_id(),
                cx_ref.get_sysno(),
                cx_ref.sepc
            );

            cx_ref.syscall_ok();
            let result = self
                .syscall(cx_ref.get_sysno(), cx_ref.args())
                .await
                .map_or_else(|e| -e.into_raw() as isize, |x| x as isize)
                as usize;

            info!(
                "[task {:?}] syscall result: {}",
                self.task.get_task_id(),
                result as isize
            );

            cx_ref[TrapFrameArgs::RET] = result;
        }
        UserTaskControlFlow::Continue
    }


    pub async fn syscall(&mut self, call_id: usize, _args: [usize; 6]) -> Result<usize, TaskError> {
        info!("[syscall] id: {}, args: {:?}", call_id, _args);
        match call_id {
            sysnum::SYS_EXIT => self.sys_exit(_args[0].try_into().unwrap()).await,
            sysnum::SYS_BRK => self.sys_brk(_args[0]).await,
            sysnum::SYS_WRITE => self.sys_write(_args[0], _args[1].into(), _args[2]).await,
            sysnum::SYS_CLONE => self.sys_clone(_args[0], _args[1], _args[2], _args[3], _args[4], _args[5]).await,
            
            _ => {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop {}
            }
        }
    }
}
