use crate::timeval::TimeVal;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ITimerVal {
    /// 定时器周期
    pub it_interval: TimeVal,
    /// 当前值（倒计时）
    pub it_value: TimeVal,
}

/// 定时器类型
#[repr(usize)]
pub enum TimerType {
    /// 实时定时器，使用真实时间（墙上时钟时间），到期发送SIGALRM信号
    ITIMER_REAL = 0,
    /// 虚拟定时器，仅当进程在用户模式下运行时计时，到期发送SIGVTALRM信号
    ITIMER_VIRTUAL = 1,
    /// 分析定时器，在进程在用户模式和内核模式下运行时计时，到期发送SIGPROF信号
    ITIMER_PROF = 2,
}

impl ITimerVal {
    pub fn new() -> Self {
        Self {
            it_interval: TimeVal::default(),
            it_value: TimeVal::default(),
        }
    }
} 