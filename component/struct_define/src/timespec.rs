#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]

pub struct TimeSpec {
    pub sec: usize,
    pub nsec: usize,
}

impl TimeSpec {
    pub fn to_nsec(&self) -> usize {
        self.sec * 1_000_000_000 + self.nsec
    }
}
