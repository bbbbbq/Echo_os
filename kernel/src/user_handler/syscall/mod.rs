use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
use trap::trap::EscapeReason;
use trap::trap::run_user_task;
use trap::trapframe::{TrapFrame, TrapFrameArgs};
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;

use crate::user_handler::userbuf::UserBuf;
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
            sysnum::SYS_CLOSE => self.sys_close(_args[0]).await,
            sysnum::SYS_MKDIRAT => {
                let dirfd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let mode = _args[2];
                self.sys_mkdirat(dirfd, &path.read_string(), mode).await
            }
            sysnum::SYS_CHDIR => {
                let path = UserBuf::new(_args[0] as *mut u8);
                self.sys_chdir(&path.read_string()).await
            }
            sysnum::SYS_OPENAT => {
                let dir_fd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let flags = _args[2];
                let mode = _args[3];
                self.sys_openat(dir_fd, &path.read_string(), flags, mode)
                    .await
            }
            sysnum::SYS_GETCWD => {
                let buf_ptr = _args[0].into();
                let size = _args[1];
                self.sys_getcwd(buf_ptr, size).await
            }
            sysnum::SYS_DUP => {
                let oldfd = _args[0];
                self.sys_dup(oldfd).await
            }
            sysnum::SYS_DUP3 => {
                let oldfd = _args[0];
                let newfd = _args[1];
                self.sys_dup3(oldfd, newfd).await
            }
            sysnum::SYS_CLONE => {
                let flags = _args[0];
                let stack = _args[1];
                let ptid = UserBuf::new(_args[2] as *mut u32);
                let tls = _args[3];
                let ctid = UserBuf::new(_args[4] as *mut u32);
                self.sys_clone(flags, stack, ptid, tls, ctid).await
            }
            sysnum::SYS_WAIT4 => {
                let pid = _args[0] as isize;
                let status = UserBuf::new(_args[1] as *mut i32);
                let options = _args[2];
                self.sys_wait4(pid, status, options).await
            }
            sysnum::SYS_EXECVE => {
                let filename = UserBuf::new(_args[0] as *mut u8);
                let argv: UserBuf<UserBuf<u8>> = UserBuf::new(_args[1] as *mut UserBuf<u8>);
                let envp: UserBuf<UserBuf<u8>> = UserBuf::new(_args[2] as *mut UserBuf<u8>);
                self.sys_execve(filename, argv, envp).await
            }
            _ => {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop {}
            }
        }
    }
}
