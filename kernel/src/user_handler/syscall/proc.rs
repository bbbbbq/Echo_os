use crate::executor::error::TaskError;
use crate::user_handler::handler::UserHandler;
use log::{debug, warn};
use crate::executor::ops::yield_now;
use crate::executor::task::{AsyncTask, CloneFlags};
use crate::user_handler::userbuf::UserBuf;
use crate::executor::thread::UserTask;
use alloc::sync::Arc;
use crate::executor::executor::GLOBLE_EXECUTOR;
use crate::executor::task::AsyncTaskItem;
use crate::executor::executor::add_ready_task;
use crate::user_handler::entry::user_entry;
use crate::executor::sync::WaitPid;
use trap::trapframe::TrapFrameArgs;
use crate::executor::id_alloc::TaskId;
use alloc::string::String;
use alloc::vec::Vec;
use filesystem::path::Path;
use crate::executor::thread::add_user_task;
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
}
