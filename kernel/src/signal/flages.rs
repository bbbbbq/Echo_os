use bitflags::bitflags;


macro_rules! bit {
    ($x:expr) => {
        1 << $x
    };
}


//!
//! 信号标志与信号相关类型定义。
//!
//! 提供信号位图、信号掩码、信号处理行为等。
bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct SignalFlags: u64 {
        /// 挂起（SIGHUP）
        const	SIGHUP		= bit!(0);
        /// 交互中断（SIGINT）
        const	SIGINT		= bit!(1);
        /// 退出（SIGQUIT）
        const	SIGQUIT		= bit!(2);
        /// 非法指令（SIGILL）
        const	SIGILL		= bit!(3);
        /// 调试陷阱（SIGTRAP）
        const	SIGTRAP		= bit!(4);
        /// IOT 指令/abort（SIGABRT）
        const	SIGABRT		= bit!(5);
        /// 总线错误（SIGBUS）
        const	SIGBUS		= bit!(6);
        /// 算术错误（SIGFPE）
        const	SIGFPE		= bit!(7);
        /// 被杀死（SIGKILL）
        const	SIGKILL		= bit!(8);
        /// 用户自定义信号 1（SIGUSR1）
        const	SIGUSR1		= bit!( 9);
        /// 无效存储访问（SIGSEGV）
        const	SIGSEGV		= bit!(10);
        /// 用户自定义信号 2（SIGUSR2）
        const	SIGUSR2		= bit!(11);
        /// 管道破裂（SIGPIPE）
        const	SIGPIPE		= bit!(12);
        /// 闹钟（SIGALRM）
        const	SIGALRM		= bit!(13);
        /// 终止请求（SIGTERM）
        const	SIGTERM		= bit!(14);
        const	SIGSTKFLT	= bit!(15);
        /// 子进程终止或停止（SIGCHLD）
        const	SIGCHLD		= bit!(16);
        /// 继续（SIGCONT）
        const	SIGCONT		= bit!(17);
        /// 停止（不可阻塞，SIGSTOP）
        const	SIGSTOP		= bit!(18);
        /// 键盘停止（SIGTSTP）
        const	SIGTSTP		= bit!(19);
        /// 后台读取控制终端（SIGTTIN）
        const	SIGTTIN		= bit!(20);
        /// 后台写控制终端（SIGTTOU）
        const	SIGTTOU		= bit!(21);
        /// 套接字有紧急数据（SIGURG）
        const	SIGURG		= bit!(22);
        /// 超出 CPU 时间限制（SIGXCPU）
        const	SIGXCPU		= bit!(23);
        /// 超出文件大小限制（SIGXFSZ）
        const	SIGXFSZ		= bit!(24);
        /// 虚拟定时器到期（SIGVTALRM）
        const	SIGVTALRM	= bit!(25);
        /// 分析定时器到期（SIGPROF）
        const	SIGPROF		= bit!(26);
        /// 窗口大小变化（SIGWINCH）
        const	SIGWINCH	= bit!(27);
        /// I/O 可用（SIGIO）
        const	SIGIO		= bit!(28);
        const   SIGPWR      = bit!(29);
        /// 错误的系统调用（SIGSYS）
        const   SIGSYS      = bit!(30);
        /* --- pthread 实时信号 --- */
        const   SIGTIMER    = bit!(31);
        const   SIGCANCEL   = bit!(32);
        const   SIGSYNCCALL = bit!(33);
        /* --- 其他实时信号 --- */
        const   SIGRT_3     = bit!(34);
        const   SIGRT_4     = bit!(35);
        const   SIGRT_5     = bit!(36);
        const   SIGRT_6     = bit!(37);
        const   SIGRT_7     = bit!(38);
        const   SIGRT_8     = bit!(39);
        const   SIGRT_9     = bit!(40);
        const   SIGRT_10    = bit!(41);
        const   SIGRT_11    = bit!(42);
        const   SIGRT_12    = bit!(43);
        const   SIGRT_13    = bit!(44);
        const   SIGRT_14    = bit!(45);
        const   SIGRT_15    = bit!(46);
        const   SIGRT_16    = bit!(47);
        const   SIGRT_17    = bit!(48);
        const   SIGRT_18    = bit!(49);
        const   SIGRT_19    = bit!(50);
        const   SIGRT_20    = bit!(51);
        const   SIGRT_21    = bit!(52);
        const   SIGRT_22    = bit!(53);
        const   SIGRT_23    = bit!(54);
        const   SIGRT_24    = bit!(55);
        const   SIGRT_25    = bit!(56);
        const   SIGRT_26    = bit!(57);
        const   SIGRT_27    = bit!(58);
        const   SIGRT_28    = bit!(59);
        const   SIGRT_29    = bit!(60);
        const   SIGRT_30    = bit!(61);
        const   SIGRT_31    = bit!(62);
        const   SIGRTMAX    = bit!(63);
    }
}

// BitExtensions trait 为 u64 类型提供 get_bit 方法
trait BitExtensions {
    fn get_bit(&self, index: usize) -> bool;
}

impl BitExtensions for u64 {
    fn get_bit(&self, index: usize) -> bool {
        (*self >> index) & 1 != 0
    }
}

impl SignalFlags {
    pub const fn from_num(num: usize) -> SignalFlags {
        SignalFlags::from_bits_truncate(1 << (num - 1))
    }

    #[inline]
    pub fn num(&self) -> usize {
        let bits = self.bits();

        for i in 0..64 {
            if bits.get_bit(i) {
                return i + 1;
            }
        }
        0
    }

    #[inline]
    pub fn is_real_time(&self) -> bool {
        self.bits() & 0xFFFFFFFE00000000 != 0
    }

    #[inline]
    pub fn real_time_index(&self) -> Option<usize> {
        self.is_real_time().then(|| self.num() - 32)
    }
}

/// 信号掩码操作方式。
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SigMaskHow {
    /// 阻塞信号
    Block,
    /// 解除阻塞
    Unblock,
    /// 设置掩码
    Setmask,
}

impl SigMaskHow {
    pub fn from_usize(how: usize) -> Option<Self> {
        match how {
            0 => Some(SigMaskHow::Block),
            1 => Some(SigMaskHow::Unblock),
            2 => Some(SigMaskHow::Setmask),
            _ => None,
        }
    }
}

/// 信号掩码。
#[derive(Debug, Clone, Copy)]
pub struct SigProcMask {
    /// 掩码位图
    pub mask: usize,
}

impl SigProcMask {
    pub fn new() -> Self {
        Self { mask: 0 }
    }

    pub fn handle(&mut self, how: SigMaskHow, mask: &Self) {
        self.mask = match how {
            SigMaskHow::Block => self.mask | mask.mask,
            SigMaskHow::Unblock => self.mask & (!mask.mask),
            SigMaskHow::Setmask => mask.mask,
        }
    }

    pub fn masked(&self, signum: usize) -> bool {
        (self.mask >> signum) & 1 == 0
    }
}

// musl riscv Sigaction
// struct Sigaction {
//     void (*handler)(int);
//     unsigned long flags;
//     void (*restorer)(void);
//     unsigned mask[2];
//     void *unused;
// }

/// 信号处理动作。
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    /// 信号处理函数指针
    pub handler: usize,    // void     (*sa_handler)(int);
    /// 信号处理标志
    pub flags: usize,      // int        sa_flags;
    /// 恢复函数指针
    pub restorer: usize,   // void     (*sa_restorer)(void);
    /// 信号掩码
    pub mask: SigProcMask, // sigset_t   sa_mask;
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            handler: 0,
            mask: SigProcMask::new(),
            flags: 0,
            restorer: 0,
        }
    }
}

// sigset_t sa_mask 是一个信号集，在调用该信号捕捉函数之前，将需要block的信号加入这个sa_mask，
// 仅当信号捕捉函数正在执行时，才阻塞sa_mask中的信号，当从信号捕捉函数返回时进程的信号屏蔽字复位为原先值。
