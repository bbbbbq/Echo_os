#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]

pub struct TimeSpec {
    pub sec: usize,
    pub nsec: usize,
}
