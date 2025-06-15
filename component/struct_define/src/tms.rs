#[derive(Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct TMS {
    pub utime: u64,
    pub stime: u64,
    pub cutime: u64,
    pub cstime: u64,
}

impl core::fmt::Display for TMS {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "TMS {{ utime: {}, stime: {}, cutime: {}, cstime: {} }}",
            self.utime, self.stime, self.cutime, self.cstime
        )
    }
}