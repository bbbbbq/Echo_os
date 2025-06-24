use crate::executor::error::TaskError;
use crate::signal::flages::{SigAction, SignalFlags};
use crate::signal::{SigMaskHow, SigProcMask};
use crate::user_handler::userbuf::UserBuf;
use crate::user_handler::syscall::UserHandler;
use log::debug;

//!
//! 信号相关系统调用实现。
//!
//! 提供 sigprocmask/sigaction 等信号处理相关的异步系统调用。

impl UserHandler {
    /// 设置信号掩码（sigprocmask 系统调用）。
    ///
    /// # 参数
    /// - `how`: 操作方式
    /// - `set`: 新掩码
    /// - `oldset`: 旧掩码
    pub async fn sys_sigprocmask(
        &self,
        how: usize,
        set: UserBuf<SigProcMask>,
        oldset: UserBuf<SigProcMask>,
    ) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_sigprocmask @ how: {:#x}, set: {}, oldset: {}",
            self.tid, how, set, oldset
        );
        
        let how = SigMaskHow::from_usize(how).ok_or(TaskError::EINVAL)?;
        let mut tcb = self.task.tcb.write();
        if oldset.is_valid() {
            oldset.write(tcb.sigmask);
        }
        if set.is_valid() {
            let sigmask = set.read();
            tcb.sigmask.handle(how, &sigmask);
        }
        drop(tcb);
        // Err(LinuxError::EPERM)
        Ok(0)
    }

    /// 设置信号处理动作（sigaction 系统调用）。
    ///
    /// # 参数
    /// - `sig`: 信号编号
    /// - `act`: 新动作
    /// - `oldact`: 旧动作
    pub async fn sys_sigaction(
        &self,
        sig: usize,
        act: UserBuf<SigAction>,
        oldact: UserBuf<SigAction>,
    ) ->  Result<usize, TaskError> {
        let signal = SignalFlags::from_num(sig);
        debug!(
            "sys_sigaction @ sig: {:?}, act: {}, oldact: {}",
            signal, act, oldact
        );
        if oldact.is_valid() {
            oldact.write(self.task.pcb.lock().sigaction[sig]);
        }
        if act.is_valid() {
            self.task.pcb.lock().sigaction[sig] = act.read();
        }
        Ok(0)
    }
}