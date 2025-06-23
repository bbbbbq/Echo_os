use crate::executor::executor::get_cur_usr_task;
use crate::executor::{executor::get_cur_task, thread::UserTask};
use crate::executor::id_alloc::TaskId;
use crate::executor::task::AsyncTask;
use crate::signal::flages::SignalFlags;
use crate::signal::SignalUserContext;
use crate::user_handler::userbuf::UserBuf;

use alloc::sync::Arc;
use timer::get_time;
use log::{debug, info};
use trap::trapframe::TrapFrameArgs;
use core::mem::size_of;
use core::fmt;

pub enum UserTaskControlFlow {
    Continue,
    Break,
}

impl fmt::Display for UserTaskControlFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_str = match self {
            UserTaskControlFlow::Continue => "Continue",
            UserTaskControlFlow::Break => "Break",
        };
        write!(f, "{}", display_str)
    }
}

pub struct UserHandler {
    pub task: Arc<UserTask>,
    pub tid: TaskId,
}

impl UserHandler {
    pub fn check_thread_exit(&self) -> Option<usize> {
        self.task
            .exit_code()
            .or(self.task.tcb.read().thread_exit_code.map(|x| x as usize))
    }

    pub fn check_timer(&self) {
        let pcb = self.task.pcb.lock();
        if let Some(timeout) = pcb.time {
            let now = get_time();
            if now >= timeout {
                info!("timer expired");
                loop{};
            }
        }
    }

    pub async fn handle_signal(&mut self, signal: SignalFlags) {
        debug!(
            "[handle_signal] enter: signal={:?} task_id={:?}",
            signal,
            self.task.get_task_id()
        );
        let entry_sepc = {
            let tcb = self.task.tcb.read();
            tcb.cx[TrapFrameArgs::SEPC]
        };
        debug!("[handle_signal] current sepc before processing: {:#x}", entry_sepc);

        debug!(
            "handle signal: {:?} task_id: {:?}",
            signal,
            self.task.get_task_id()
        );

        // if the signal is SIGKILL, then exit the task immediately.
        // the SIGKILL can't be catched and be ignored.
        if signal == SignalFlags::SIGKILL {
            self.task.exit_with_signal(signal.num());
        }

        // get the signal action for the signal.
        let sigaction = self.task.pcb.lock().sigaction[signal.num()].clone();

        if sigaction.handler == 0 {
            match signal {
                SignalFlags::SIGCANCEL | SignalFlags::SIGSEGV | SignalFlags::SIGILL => {
                    get_cur_usr_task().unwrap().exit_with_signal(signal.num());
                }
                _ => {}
            }
            return;
        }
        // ignore signal if the handler of is SIG_IGN(1)
        if sigaction.handler == 1 {
            return;
        }

        info!(
            "handle signal: {:?} task: {:?}",
            signal,
            self.task.get_task_id()
        );

        // let cx_ref = unsafe { task.get_cx_ptr().as_mut().unwrap() };
        let cx_ref = self.task.force_cx_ref();
        // store task_mask and context.
        let task_mask = self.task.tcb.read().sigmask;
        let store_cx = cx_ref.clone();
        // 更新线程的 sigmask，完整复制掩码结构
        self.task.tcb.write().sigmask = sigaction.mask;

        // alloc space for SignalUserContext at stack and align with 16 bytes.
        let sp = (cx_ref[TrapFrameArgs::SP] - 128 - size_of::<SignalUserContext>()) / 16 * 16;
        // 通过 UserBuf 新建用户缓冲区并获取可变引用，保持 UserBuf 生命周期
        let ub = UserBuf::<SignalUserContext>::new(sp as *mut _);
        let cx: &mut SignalUserContext = ub.get_mut();
        // change task context to do the signal.
        let mut tcb = self.task.tcb.write();
        cx.store_ctx(&cx_ref);
        cx.set_pc(tcb.cx[TrapFrameArgs::SEPC]);
        // 复制掩码位到信号上下文
        cx.sig_mask.mask = sigaction.mask.mask;
        tcb.cx[TrapFrameArgs::SP] = sp;
        tcb.cx[TrapFrameArgs::SEPC] = sigaction.handler;
        // 若用户未指定 restorer，则动态在用户栈生成一个执行 sys_sigreturn 的跳板，
        // 避免返回地址为 0 触发页错误。
        tcb.cx[TrapFrameArgs::RA] = if sigaction.restorer == 0 {
            // SIG_RETURN_ADDR
            // TODO: add sigreturn addr.
            0
        } else {
            sigaction.restorer
        };
        tcb.cx[TrapFrameArgs::ARG0] = signal.num();
        tcb.cx[TrapFrameArgs::ARG1] = 0;
        tcb.cx[TrapFrameArgs::ARG2] = cx as *mut SignalUserContext as usize;
        drop(tcb);

        loop {
            if let Some(exit_code) = self.task.exit_code() {
                debug!(
                    "program exit with code: {}  task_id: {:?}",
                    exit_code,
                    self.task.get_task_id()
                );
                break;
            }

            let cx_ref = self.task.force_cx_ref();

            debug!(
                "[task {:?}]task sepc: {:#x}",
                self.task.get_task_id(),
                cx_ref[TrapFrameArgs::SEPC]
            );
            let res = self.handle_syscall(cx_ref).await;
            debug!("[task {:?}] syscall result: {}", self.task.get_task_id(), res);
            if let UserTaskControlFlow::Break = res {
                return;
            }
        }
        info!(
            "[handle_signal] finished signal={:?} task={:?}",
            signal,
            self.task.get_task_id()
        );
        debug!("[handle_signal] store_cx.sepc={:#x}  cx.pc()={:#x}", store_cx[TrapFrameArgs::SEPC], cx.pc());
        // restore sigmask to the mask before doing the signal.
        self.task.tcb.write().sigmask = task_mask;
        *cx_ref = store_cx;
        // copy pc from new_pc
        cx_ref[TrapFrameArgs::SEPC] = cx.pc();
        cx.restore_ctx(cx_ref);
        debug!("[handle_signal] after restore_ctx sepc={:#x}", cx_ref[TrapFrameArgs::SEPC]);
    }
}
