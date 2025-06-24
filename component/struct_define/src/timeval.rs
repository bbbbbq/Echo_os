use core::time::Duration;



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
