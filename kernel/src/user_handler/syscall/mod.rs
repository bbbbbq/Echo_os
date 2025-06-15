use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
use struct_define::tms::TMS;
use struct_define::uname::UTSname;
use trap::trap::EscapeReason;
use trap::trap::run_user_task;
use trap::trapframe::{TrapFrame, TrapFrameArgs};
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;
use crate::user_handler::syscall::other::TimeVal;
use crate::user_handler::userbuf::UserBuf;
use log::error;
use log::info;
use filesystem::file::Stat;
pub mod fs;
pub mod mem;
pub mod proc;
pub mod other;
use struct_define::timespec::TimeSpec;

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
            sysnum::SYS_EXIT => self.sys_exit(_args[0].try_into().unwrap_or(1)).await,
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
                self.sys_openat(dir_fd as usize, path, flags, mode)
                    .await
                    .map(|x| x as usize)
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
            sysnum::SYS_FSTAT => {
                let fd = _args[0];
                self.sys_fstat(fd, UserBuf::new(_args[1] as *mut Stat))
                    .await
                    .map(|x| x as usize)
            }
            sysnum::SYS_GETDENTS64 => {
                let fd = _args[0];
                let dirp = UserBuf::new(_args[1] as *mut u8);
                let count = _args[2];
                self.sys_getdents64(fd, dirp, count).await
            }
            sysnum::SYS_GETPID => {
                self.sys_getpid().await
            }
            sysnum::SYS_GETPPID => {
                self.sys_getppid().await
            }
            sysnum::SYS_GETTIMEOFDAY => {
                let tv_ptr = UserBuf::new(_args[0] as *mut TimeVal);
                let timezone_ptr = _args[1];
                self.sys_gettimeofday(tv_ptr, timezone_ptr).await
            }
            sysnum::SYS_MMAP => {
                let addr = _args[0];
                let len = _args[1];
                let prot = _args[2];
                let flags = _args[3];
                let fd = _args[4];
                let offset = _args[5];
                self.sys_mmap(addr, len, prot, flags, fd, offset).await
            }
            sysnum::SYS_MUNMAP => {
                let addr = _args[0];
                let len = _args[1];
                self.sys_munmap(addr, len).await
            }
            sysnum::SYS_READ => {
                let fd = _args[0];
                let buf_ptr = UserBuf::new(_args[1] as *mut u8);
                let count = _args[2];
                self.sys_read(fd, buf_ptr, count).await
            }
            sysnum::SYS_PIPE2 => {
                let fds_ptr = UserBuf::new(_args[0] as *mut u32);
                let unknown = _args[1];
                self.sys_pipe2(fds_ptr, unknown).await
            }
            sysnum::SYS_NANOSLEEP => {
                let req = UserBuf::new(_args[0] as *mut TimeSpec);
                let _rem = UserBuf::new(_args[1] as *mut TimeSpec);
                self.sys_nanosleep(req, _rem).await
            }
            sysnum::SYS_TIMES => {
                let tms_ptr = UserBuf::new(_args[0] as *mut TMS);
                self.sys_times(tms_ptr).await
            }
            sysnum::SYS_UNAME => self.sys_uname(UserBuf::new(_args[0] as *mut UTSname)).await,
            _ => {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop {}
            }

        }
    }
}
