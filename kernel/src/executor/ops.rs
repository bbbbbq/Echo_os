use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use timer::current_nsec;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

// 全局睡眠状态管理
struct SleepState {
    waker: Option<Waker>,
    woken: AtomicBool,
}

impl SleepState {
    fn new() -> Self {
        Self {
            waker: None,
            woken: AtomicBool::new(false),
        }
    }
}

lazy_static! {
    static ref SLEEP_STATE: Mutex<SleepState> = Mutex::new(SleepState::new());
}

pub struct Yield(bool);

impl Yield {
    pub const fn new() -> Self {
        Self(false)
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0 {
            true => Poll::Ready(()),
            false => {
                self.0 = true;
                Poll::Pending
            }
        }
    }
}

pub async fn yield_now() {
    // 减少日志输出，避免干扰
    // info!("Yielding to other tasks");
    Yield::new().await;
}

pub struct Sleep {
    wake_time: usize,
    first_poll: bool,
}

impl Sleep {
    pub fn new(wake_time: usize) -> Self {
        Self { 
            wake_time, 
            first_poll: true 
        }
    }
    
    // 检查是否该醒来
    fn should_wake(&self) -> bool {
        current_nsec() >= self.wake_time
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 如果时间到了，就醒来
        if self.should_wake() {
            return Poll::Ready(());
        }
        
        // 第一次 poll，注册 waker
        if self.first_poll {
            self.first_poll = false;
            
            // 保存 waker，用于之后唤醒
            let mut state = SLEEP_STATE.lock();
            state.waker = Some(cx.waker().clone());
            state.woken.store(false, Ordering::SeqCst);
            
            // 计算需要睡眠的时间
            let time_to_sleep = self.wake_time.saturating_sub(current_nsec());
            
            // 如果睡眠时间很短，直接自旋等待
            if time_to_sleep < 500_000 { // 小于0.5毫秒
                // 自旋等待
                while !self.should_wake() {
                    core::hint::spin_loop();
                }
                return Poll::Ready(());
            }
            
            // 对于短时间睡眠但大于0.5毫秒，使用短暂自旋后再让出
            if time_to_sleep < 10_000_000 { // 10毫秒内
                // 自旋一小段时间，提高响应性
                for _ in 0..1000 {
                    core::hint::spin_loop();
                }
            }
        }
        
        // 返回Pending，等待下次被调度
        Poll::Pending
    }
}

// 特殊处理终端输入等待的函数
pub async fn terminal_wait(duration_ms: usize) {
    // 对于终端输入等待，使用短自旋+让出执行权的策略
    // 这样可以保持较高的响应性同时不会消耗过多CPU
    
    // 自旋次数根据等待时间调整
    let spin_count = if duration_ms <= 10 {
        5000 // 短等待时间使用较多自旋
    } else {
        1000 // 长等待时间使用较少自旋
    };
    
    for _ in 0..spin_count {
        core::hint::spin_loop();
    }
    
    // 让出执行权给其他任务
    yield_now().await;
}

pub async fn sleep_for_duration(duration_ms: usize) {
    // 最小休眠时间为 5ms
    let duration_ms = duration_ms.max(5);
    let wake_time = current_nsec() + duration_ms * 1_000_000;
    Sleep::new(wake_time).await;
}
