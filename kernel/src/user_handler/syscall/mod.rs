use super::handler::UserHandler;
use super::handler::UserTaskControlFlow;
use log::debug;
use struct_define::tms::TMS;
use struct_define::uname::UTSname;
use trap::trap::EscapeReason;
use trap::trap::run_user_task;
use trap::trapframe::{TrapFrame, TrapFrameArgs};
pub mod sysnum;
use crate::executor::error::TaskError;
use crate::executor::task::AsyncTask;
use crate::user_handler::syscall::other::TimeVal;
use crate::user_handler::syscall::sysnum::sysnum_to_string;
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
        //debug!("TrapFrame: {:?}", cx_ref);

        if matches!(run_user_task(cx_ref), EscapeReason::SysCall) {
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
                "[task {:?}] syscall result: {} \n\n",
                self.task.get_task_id(),
                result as isize
            );

            cx_ref[TrapFrameArgs::RET] = result;
        }
        UserTaskControlFlow::Continue
    }

    pub async fn syscall(&mut self, call_id: usize, _args: [usize; 6]) -> Result<usize, TaskError> {
        info!(" \n\n [syscall] id: {:x} {:?} , args: {:x?} ", call_id,sysnum_to_string(call_id), _args);
        match call_id {
            sysnum::SYS_FCNTL => self.sys_fcntl(_args[0], _args[1], _args[2]).await,
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
            sysnum::SYS_MOUNT => {
                let source = UserBuf::new(_args[0] as *mut u8);
                let target = UserBuf::new(_args[1] as *mut u8);
                let filesystem_type = UserBuf::new(_args[2] as *mut u8);
                let mount_flags = _args[3];
                let data = UserBuf::new(_args[4] as *mut u8);
                self.sys_mount(source, target, filesystem_type, mount_flags, data).await
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
            sysnum::SYS_FACCESSAT => {
                let dir_fd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let mode = _args[2];
                let flags = _args[3];
                self.sys_faccessat(dir_fd, path, mode, flags).await
            }
            sysnum::SYS_CLOCK_GETTIME => {
                let clock_id = _args[0];
                let times_ptr = UserBuf::new(_args[1] as *mut TimeSpec);
                self.sys_clock_gettime(clock_id, times_ptr).await
            }
            sysnum::SYS_IOCTL => {
                let fd = _args[0].try_into().unwrap();
                let request = _args[1].try_into().unwrap();
                let argp = _args[2];
                self.sys_ioctl(fd, request, argp).await
            }
            sysnum::SYS_GETUID => self.sys_getuid().await,
            sysnum::SYS_UNAME => self.sys_uname(UserBuf::new(_args[0] as *mut UTSname)).await,
            sysnum::SYS_SCHED_YIELD => self.sys_sched_yield().await,
            sysnum::SYS_SET_TID_ADDRESS => self.sys_set_tid_address(UserBuf::new(_args[0] as *mut u32)).await,
            sysnum::SYS_FSTATAT => {
                let dir_fd = _args[0] as isize;
                let path = UserBuf::new(_args[1] as *mut u8);
                let statbuf = UserBuf::new(_args[2] as *mut u8);
                let flags = _args[3];
                self.sys_fstatat(dir_fd, path, statbuf, flags).await
            }
            sysnum::SYS_EXIT_GROUP => {
                let exit_code = _args[0];
                self.sys_exit_group(exit_code).await
            }
            _ => {
                info!("call_id : {}", call_id);
                error!("Invalid syscall");
                loop {}
            }
        }
    }
}
