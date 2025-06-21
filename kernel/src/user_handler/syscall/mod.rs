use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
use struct_define::iov::IoVec;
use struct_define::poll_event::PollFd;
use struct_define::tms::TMS;
use struct_define::uname::UTSname;
use struct_define::rlimit::Rlimit;
use trap::trap::EscapeReason;
use trap::trap::run_user_task;
use trap::trapframe::{TrapFrame, TrapFrameArgs};
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;
use crate::signal::flages::SigAction;
use crate::signal::SigProcMask;
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
pub mod signal;

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
            sysnum::SYS_GETPGID => {
                self.sys_getpgid().await
            }
            sysnum::SYS_GETEUID => {
                self.sys_geteuid().await
            }
            sysnum::SYS_SETPGID => {
                self.sys_setpgid(_args[0] as usize, _args[1] as usize).await
            }
            sysnum::SYS_SIGACTION => {
                let sig = _args[0];
                let act = UserBuf::new(_args[1] as *mut SigAction);
                let oldact = UserBuf::new(_args[2] as *mut SigAction);
                self.sys_sigaction(sig, act, oldact).await
            }
            sysnum::SYS_SIGPROCMASK => {
                let how = _args[0];
                let set = UserBuf::new(_args[1] as *mut SigProcMask);
                let oldset = UserBuf::new(_args[2] as *mut SigProcMask);
                self.sys_sigprocmask(how, set, oldset).await
            }
            sysnum::SYS_GETCWD => {
                let buf_ptr = UserBuf::new(_args[0] as *mut u8);
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
            sysnum::SYS_MOUNT => {
                let source = UserBuf::new(_args[0] as *mut u8);
                let target = UserBuf::new(_args[1] as *mut u8);
                let filesystem_type = UserBuf::new(_args[2] as *mut u8);
                let mount_flags = _args[3];
                let data = UserBuf::new(_args[4] as *mut u8);
                self.sys_mount(source, target, filesystem_type, mount_flags, data).await
            }
            sysnum::SYS_WRITEV => {
                let fd = _args[0];
                let iov = UserBuf::new(_args[1] as *mut IoVec);
                let iocnt = _args[2];
                self.sys_writev(fd, iov, iocnt).await
            }
            sysnum::SYS_FCNTL => {
                let fd = _args[0];
                let cmd = _args[1];
                let arg = _args[2];
                self.sys_fcntl(fd, cmd, arg).await
            }
            sysnum::SYS_UMOUNT2 => {
                let target = UserBuf::new(_args[0] as *mut u8);
                self.sys_umount(target).await
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
            sysnum::SYS_UNLINKAT => {
                let dir_fd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let flags = _args[2];
                self.sys_unlinkat(dir_fd, path, flags).await
            }
            sysnum::SYS_KILL => {
                let pid = _args[0] as usize;
                let signum = _args[1];
                self.sys_kill(pid, signum).await
            }
            sysnum::SYS_IOCTL => {
                let fd = _args[0];
                let request = _args[1];
                let arg1 = _args[2];
                let arg2 = _args[3];
                let arg3 = _args[4];
                self.sys_ioctl(fd, request, arg1, arg2, arg3).await
            }
            sysnum::SYS_CLOCK_GETTIME => {
                let clock_id = _args[0];
                let times_ptr = UserBuf::new(_args[1] as *mut TimeSpec);
                self.sys_clock_gettime(clock_id, times_ptr).await
            }
            sysnum::SYS_FSTATAT => {
                let dir_fd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let stat_ptr = UserBuf::new(_args[2] as *mut Stat);
                self.sys_fstatat(dir_fd, path, stat_ptr).await
            }
            sysnum::SYS_EXIT_GROUP => {
                let exit_code = _args[0] as isize;
                self.sys_exit_group(exit_code).await
            }
            sysnum::SYS_PPOLL => {
                let poll_fds_ptr = UserBuf::new(_args[0] as *mut PollFd);
                let nfds = _args[1];
                let timeout_ptr = UserBuf::new(_args[2] as *mut TimeSpec);
                let sigmask_ptr = _args[3];
                self.sys_ppoll(poll_fds_ptr, nfds, timeout_ptr, sigmask_ptr).await
            }
            sysnum::SYS_GETUID => self.sys_getuid().await,
            sysnum::SYS_UNAME => self.sys_uname(UserBuf::new(_args[0] as *mut UTSname)).await,
            sysnum::SYS_SCHED_YIELD => self.sys_sched_yield().await,
            sysnum::SYS_SET_TID_ADDRESS => self.sys_set_tid_address(UserBuf::new(_args[0] as *mut u32)).await,
            sysnum::SYS_SET_ROBUST_LIST => {
                let head_ptr = _args[0];
                let len = _args[1];
                self.sys_set_robust_list(head_ptr, len).await
            }
            sysnum::SYS_PRLIMIT64 => {
                let pid = _args[0];
                let resource = _args[1];
                let new_limit = UserBuf::new(_args[2] as *mut Rlimit);
                let old_limit = UserBuf::new(_args[3] as *mut Rlimit);
                self.sys_prlimit64(pid, resource, new_limit, old_limit).await
            }
            sysnum::SYS_READLINKAT => {
                let dirfd = _args[0] as isize;
                let pathname = UserBuf::new(_args[1] as *mut u8);
                let buf = UserBuf::new(_args[2] as *mut u8);
                let bufsiz = _args[3];
                self.sys_readlinkat(dirfd, pathname, buf, bufsiz).await
            }
            sysnum::SYS_GETRANDOM => {
                let buf = UserBuf::new(_args[0] as *mut u8);
                let buflen = _args[1];
                let flags = _args[2];
                self.sys_getrandom(buf, buflen, flags).await
            }
            _ => {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop {}
            }
        }
    }
}
