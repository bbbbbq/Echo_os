use trap::trap::run_user_task;
use trap::trap::EscapeReason;
use trap::trap::TrapFrame;
use trap::trapframe::TrapFrameArgs;
use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::user_handler::syscall::sysnum::SYS_WRITE;
use log::debug;
use crate::executor::task::AsyncTask;
use log::error;
use log::info;
pub mod fs;

impl UserHandler
{
    pub async fn handle_syscall(&self, cx_ref: &mut TrapFrame) -> UserTaskControlFlow {
        if matches!(run_user_task(cx_ref), EscapeReason::SysCall) {
            if cx_ref.get_sysno() == sysnum::SYS_SIGRETURN as _ {
                return UserTaskControlFlow::Break;
            }
            cx_ref.syscall_ok();
            let result = self
            .syscall(cx_ref.get_sysno(), cx_ref.args())
            .await
            .map_or_else(|e| -e.into_raw() as isize, |x| x as isize)
            as usize;

            debug!(
                "[task {:?}] syscall result: {}",
                self.task.get_task_id(),
                result as isize
            );

            cx_ref[TrapFrameArgs::RET] = result;
        }
        UserTaskControlFlow::Continue
    }

    
    pub async fn syscall(&self, call_id: usize, _args: [usize; 6]) -> Result<usize, TaskError> {
        match call_id {
            sysnum::SYS_EXIT => {
               unimplemented!()
            }
            sysnum::SYS_WRITE => 
            {
                self.sys_write(_args[0], _args[1].into(), _args[2]).await
            }
            _ => 
            {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop{}
            },
        }
    }
}
