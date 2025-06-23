use crate::executor::error::TaskError;
use crate::executor::ops::yield_now;
use crate::signal::flages::SignalFlags;
use crate::user_handler::handler::UserHandler;
use log::{debug, warn};
use crate::executor::task::{AsyncTask, CloneFlags};
use crate::user_handler::userbuf::UserBuf;
use crate::executor::task::AsyncTaskItem;
use crate::executor::executor::{add_ready_task, tid2task};
use crate::user_handler::entry::user_entry;
use crate::executor::sync::WaitPid;
use trap::trapframe::TrapFrameArgs;
use crate::executor::id_alloc::TaskId;
use alloc::string::String;
use alloc::vec::Vec;
use filesystem::path::Path;
use crate::executor::thread::{add_user_task, UserTask};
use struct_define::rlimit::Rlimit;
impl UserHandler {
    pub async fn sys_exit(&self, exit_code: isize) -> Result<usize, TaskError> {
        debug!(
            "sys_exit @ exit_code: {}  task_id: {:?}",
            exit_code, self.tid
        );
        self.task.thread_exit(exit_code as _);
        Ok(0)
    }

    pub async fn sys_clone(
        &self,
        flags: usize,
        stack: usize,
        ptid: UserBuf<u32>,
        tls: usize,
        ctid: UserBuf<u32>,
    ) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_clone @ flags: {:#x}, stack: {:#x}, ptid: {:?}, tls: {:#x}, ctid: {:?}",
            self.task.get_task_id(), flags, stack, ptid, tls, ctid
        );
        let flags = CloneFlags::from_bits_truncate(flags);
        debug!(
            "[task {:?}] sys_clone @ flags: {:?}, stack: {:#x}, ptid: {:?}, tls: {:#x}, ctid: {:?}",
            self.task.get_task_id(), flags, stack, ptid, tls, ctid
        );
        
        let new_task = if flags.contains(CloneFlags::THREAD) {
            self.task.thread_clone()
        } else {
            self.task.clone().process_clone()
        };

        if stack != 0 {
            new_task.tcb.write().cx[TrapFrameArgs::SP] = stack;
        }
        // set tls.
        if flags.contains(CloneFlags::SETTLS) {
            new_task.tcb.write().cx[TrapFrameArgs::TLS] = tls;
        }

        // TODO: handle ptid and ctid
        
        let new_task_id = new_task.get_task_id();
        add_ready_task(AsyncTaskItem::new(new_task, user_entry()));
        yield_now().await;
        Ok(new_task_id.0)
    }

    pub async fn sys_wait4(
        &self,
        pid: isize,           // 指定进程ID，可为-1等待任何子进程；
        status: UserBuf<i32>, // 接收状态的指针；
        options: usize,       // WNOHANG，WUNTRACED，WCONTINUED；
    ) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_wait4 @ pid: {}, status: {:?}, options: {}",
            self.tid, pid, status, options
        );

        // return LinuxError::ECHILD if there has no child process.
        if self.task.inner_map(|inner| inner.children.len()) == 0 {
            return Err(TaskError::ECHILD);
        }

        if pid != -1 {
            self.task
                .inner_map(|inner| {
                    inner
                        .children
                        .iter()
                        .find(|x| x.task_id == TaskId(pid as usize))
                        .map(|x| x.clone())
                })
                .ok_or(TaskError::ECHILD)?;
        }
        if options == 0 || options == 2 || options == 3 || options == 10 {
            debug!(
                "children:{:?}",
                self.task.pcb.lock().children.iter().count()
            );
            let child_task = WaitPid(self.task.clone(), pid).await?;

            debug!(
                "wait ok: {:?}  waiter: {:?}",
                child_task.task_id, self.task.task_id
            );
            // release the task resources
            self.task
                .pcb
                .lock()
                .children
                .retain(|x| x.task_id != child_task.task_id);
            child_task.release();
            debug!("wait pid: {}", child_task.exit_code().unwrap());

            if status.is_valid() {
                status.write((child_task.exit_code().unwrap() as i32) << 8);
            }
            Ok(child_task.task_id.0)
        } else if options == 1 {
            let child_task = self
                .task
                .pcb
                .lock()
                .children
                .iter()
                .find(|x| x.task_id == TaskId(pid as usize) || pid == -1)
                .cloned();
            let exit = child_task.clone().map_or(None, |x| x.exit_code());
            match exit {
                Some(t1) => {
                    let child_task = child_task.unwrap();
                    // Release task.
                    self.task
                        .pcb
                        .lock()
                        .children
                        .retain(|x| x.task_id != child_task.task_id);
                    child_task.release();
                    if status.is_valid() {
                        status.write((t1 as i32) << 8);
                    }
                    // TIPS: This is a small change.
                    Ok(child_task.task_id.0)
                    // Ok(0)
                }
                None => Ok(0),
            }
        } else {
            warn!("wait4 unsupported options: {}", options);
            Err(TaskError::EPERM)
        }
    }

    pub async fn sys_execve(
        &self,
        filename: UserBuf<u8>,      // *mut i8
        args: UserBuf<UserBuf<u8>>, // *mut *mut i8
        envp: UserBuf<UserBuf<u8>>, // *mut *mut i8
    ) -> Result<usize, TaskError> {
        let file_name = filename.read_string();
        let mut args_vec: Vec<String> = Vec::new();
        let mut args_ptr = args;
        while args_ptr.is_valid() {
            let arg_ptr = args_ptr.read();
            if arg_ptr.is_valid() {
                args_vec.push(arg_ptr.read_string());
                args_ptr = args_ptr.offset(1);
            } else {
                break;
            }
        }

        let mut envp_vec: Vec<String> = Vec::new();
        let mut envp_ptr = envp;
        while envp_ptr.is_valid() {
            let env_ptr = envp_ptr.read();
            if env_ptr.is_valid() {
                envp_vec.push(env_ptr.read_string());
                envp_ptr = envp_ptr.offset(1);
            } else {
                break;
            }
        }

        debug!("sys_execve @ filename: {}, args: {:?}, envp: {:?}", file_name, args_vec, envp_vec);
                let _path = Path::new(file_name.clone());
        
        // Convert Vec<String> to Vec<&str>
        let args_str: Vec<&str> = args_vec.iter().map(|s| s.as_str()).collect();
        let envp_str: Vec<&str> = envp_vec.iter().map(|s| s.as_str()).collect();
        let id = add_user_task(&file_name, args_str, envp_str).await;
        self.task.thread_exit(id.0);
        Ok(id.0)
    }


    /// sys_getpid() 获取进程 id
    pub async fn sys_getpid(&self) -> Result<usize, TaskError> {
        Ok(self.task.process_id.0)
    }

    pub async fn sys_getppid(&self) -> Result<usize, TaskError> {
        match self.task.parent.read().upgrade() {
            Some(parent) => Ok(parent.process_id.0),
            None => Ok(-1_isize as usize),
        }
    }

    pub async fn sys_sched_yield(&self) -> Result<usize, TaskError> {
        debug!("sys_sched_yield @ ");
        yield_now().await;
        Ok(0)
    }

    pub async fn sys_set_tid_address(&self, tid_address: UserBuf<u32>) -> Result<usize, TaskError> {
        debug!("sys_set_tid_address @ tid_address: {:?}", tid_address);
        let tid_address = tid_address.read();
        self.task.tcb.write().clear_child_tid = Some(tid_address.try_into().unwrap());
        Ok(self.tid.0)
    }

    pub async fn sys_exit_group(&self, exit_code: isize) -> Result<usize, TaskError> {
        debug!("sys_exit_group @ exit_code: {}", exit_code);
        self.task.thread_exit(exit_code as _);
        Ok(0)
    }

    pub async fn sys_geteuid(&self) -> Result<usize, TaskError> {
        Ok(0)
    }

    pub async fn sys_prlimit64(
        &self,
        pid: usize,
        resource: usize,
        new_limit: UserBuf<Rlimit>,
        old_limit: UserBuf<Rlimit>,
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_prlimit64 @ pid: {}, resource: {}, new_limit: {:?}, old_limit: {:?}",
            pid, resource, new_limit, old_limit
        );

        // Resource limit constants (from Linux)
        const RLIMIT_CPU: usize = 0;
        const RLIMIT_FSIZE: usize = 1;
        const RLIMIT_DATA: usize = 2;
        const RLIMIT_STACK: usize = 3;
        const RLIMIT_CORE: usize = 4;
        const RLIMIT_RSS: usize = 5;
        const RLIMIT_NPROC: usize = 6;
        const RLIMIT_NOFILE: usize = 7;
        const RLIMIT_MEMLOCK: usize = 8;
        const RLIMIT_AS: usize = 9;
        const RLIMIT_LOCKS: usize = 10;
        const RLIMIT_SIGPENDING: usize = 11;
        const RLIMIT_MSGQUEUE: usize = 12;
        const RLIMIT_NICE: usize = 13;
        const RLIMIT_RTPRIO: usize = 14;
        const RLIMIT_RTTIME: usize = 15;

        // Check if resource is valid
        if resource > RLIMIT_RTTIME {
            return Err(TaskError::EINVAL);
        }

        // For now, we only support getting/setting limits for the current process (pid = 0)
        if pid != 0 {
            return Err(TaskError::EINVAL);
        }

        // Default limits (simplified implementation)
        let default_limits = match resource {
            RLIMIT_CPU => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_FSIZE => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_DATA => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_STACK => Rlimit { curr: 8 * 1024 * 1024, max: usize::MAX }, // 8MB stack
            RLIMIT_CORE => Rlimit { curr: 0, max: usize::MAX },
            RLIMIT_RSS => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_NPROC => Rlimit { curr: 1024, max: 1024 },
            RLIMIT_NOFILE => Rlimit { curr: 1024, max: 1024 },
            RLIMIT_MEMLOCK => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_AS => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_LOCKS => Rlimit { curr: usize::MAX, max: usize::MAX },
            RLIMIT_SIGPENDING => Rlimit { curr: 1024, max: 1024 },
            RLIMIT_MSGQUEUE => Rlimit { curr: 819200, max: 819200 },
            RLIMIT_NICE => Rlimit { curr: 0, max: 0 },
            RLIMIT_RTPRIO => Rlimit { curr: 0, max: 0 },
            RLIMIT_RTTIME => Rlimit { curr: usize::MAX, max: usize::MAX },
            _ => return Err(TaskError::EINVAL),
        };

        // If old_limit is valid, write current limits
        if old_limit.is_valid() {
            old_limit.write(default_limits);
        }

        // If new_limit is valid, we would set the limits here
        // For now, we just validate the input but don't actually change limits
        if new_limit.is_valid() {
            let new_limits = new_limit.read();
            
            // Basic validation: current limit should not exceed maximum limit
            if new_limits.curr > new_limits.max {
                return Err(TaskError::EINVAL);
            }
            
            // TODO: Actually implement limit setting and enforcement
            // This would require storing limits in the task's PCB
        }

        Ok(0)
    }

    pub async fn sys_kill(&self, pid: usize, signum: usize) -> Result<usize, TaskError> {
        let signal = SignalFlags::from_num(signum);
        debug!(
            "[task {:?}] sys_kill @ pid: {}, signum: {:?}",
            self.task.get_task_id(), pid, signal
        );

        let user_task = match tid2task(TaskId(pid)) {
            Some(task) => task.downcast_arc::<UserTask>().map_err(|_| TaskError::ESRCH),
            None => Err(TaskError::ESRCH),
        }?;

        user_task.tcb.write().signal.add_signal(signal.clone());

        yield_now().await;

        Ok(0)
    }

    pub async fn sys_gettid(&self) -> Result<usize, TaskError> {
        Ok(self.task.get_task_id().0)
    }

}
