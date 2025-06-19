use crate::executor::sync::Sleep;
use crate::executor::task::AsyncTask;
use crate::user_handler::handler::UserHandler;
use crate::executor::error::TaskError;
use crate::user_handler::userbuf::UserBuf;

use log::{debug, error};
use struct_define::tms::TMS;
use core::time::Duration;
use timer::get_time;
use struct_define::timespec::TimeSpec;
use struct_define::uname::UTSname;


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

    pub async fn sys_nanosleep(&self, req: UserBuf<TimeSpec>, _rem: UserBuf<TimeSpec>) -> Result<usize, TaskError> {

        let req = req.read();
        let _rem = _rem.read();

        let duration = Duration::from_secs(req.sec as u64) + Duration::from_nanos(req.nsec as u64);
        let sleep = Sleep { time: duration };
        sleep.await;
        Ok(0)
    }

    pub async fn sys_times(&self, tms_ptr: UserBuf<TMS>) -> Result<usize, TaskError> {
        const CLK_TCK: u128 = 100;
        debug!("sys_times @ tms_ptr: {}", tms_ptr);
        let duration = get_time();
        let total_ticks = duration.as_nanos() * CLK_TCK / 1_000_000_000;
        let tms = TMS {
            utime: 0, // TODO: track user time
            stime: total_ticks as u64,
            cutime: 0,
            cstime: 0,
        };
        // println!("duration: {:?}", duration);
        // println!("sys_times @ tms: {:#?}", tms);
        tms_ptr.write(tms);
        Ok(0)
    }

    pub async fn sys_uname(&self, buf_ptr: UserBuf<UTSname>) -> Result<usize, TaskError> {
        debug!("sys_uname @ uts_ptr: {}", buf_ptr);

        let mut uts = UTSname::new();

        let sys_name = b"Linux";
        uts.sysname[..sys_name.len()].copy_from_slice(sys_name);

        let sys_nodename = b"debian";
        uts.nodename[..sys_nodename.len()].copy_from_slice(sys_nodename);

        let sys_release = b"5.10.0-7-riscv64";
        uts.release[..sys_release.len()].copy_from_slice(sys_release);

        let sys_version = b"#1 SMP Debian 5.10.40-1 (2021-05-28)";
        uts.version[..sys_version.len()].copy_from_slice(sys_version);

        let sys_machine = b"riscv qemu";
        uts.machine[..sys_machine.len()].copy_from_slice(sys_machine);

        // domainname is already all zeros from default(), which is a valid empty C string.

        buf_ptr.write(uts);

        Ok(0)
    }

    pub async fn sys_getuid(&self) -> Result<usize, TaskError> {
        Ok(0)
    }

    pub async fn sys_getgid(&self) -> Result<usize, TaskError> {
        Ok(0)
    }

    pub async fn sys_getpgid(&self) -> Result<usize, TaskError> {
        Ok(0)
    }

    pub async fn sys_setpgid(&self, _pid: usize, _pgid: usize) -> Result<usize, TaskError> {
        Ok(0)
    }

    pub async fn sys_clock_gettime(
        &self,
        clock_id: usize,
        times_ptr: UserBuf<TimeSpec>,
    ) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] sys_clock_gettime @ clock_id: {}, times_ptr: {}",
            self.task.get_task_id(), clock_id, times_ptr
        );

        let ns = match clock_id {
            0 => get_time(),        // CLOCK_REALTIME
            1 => get_time(), // CLOCK_MONOTONIC
            2 => {
                error!("CLOCK_PROCESS_CPUTIME_ID not implemented");
                Duration::ZERO
            }
            3 => {
                error!("CLOCK_THREAD_CPUTIME_ID not implemented");
                Duration::ZERO
            }
            _ => return Err(TaskError::EINVAL),
        };

        times_ptr.write(TimeSpec {
            sec: ns.as_secs() as usize,
            nsec: ns.subsec_nanos() as usize,
        });
        Ok(0)
    }
}