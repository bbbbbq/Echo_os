use crate::user_handler::handler::UserHandler;
use crate::executor::error::TaskError;
use crate::user_handler::userbuf::UserBuf;
use log::debug;
use core::time::Duration;
use timer::get_time;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeVal {
    /// seconds, range in 0~999999999
    pub sec: usize,
    /// microseconds, range in 0~999999
    pub usec: usize,
}

impl From<Duration> for TimeVal {
    fn from(duration: Duration) -> Self {
        TimeVal {
            sec: duration.as_secs() as usize,
            usec: (duration.subsec_micros()) as usize,
        }
    }
}



impl UserHandler {
    pub async fn sys_gettimeofday(&self, tv_ptr: UserBuf<TimeVal>, timezone_ptr: usize) -> Result<usize, TaskError> {
        debug!(
            "sys_gettimeofday @ tv_ptr: {}, timezone: {:#x}",
            tv_ptr, timezone_ptr
        );
        let time= get_time();
        tv_ptr.write(time.into());
        Ok(0)
    }
}