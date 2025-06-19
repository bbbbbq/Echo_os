
#[repr(C)]
#[derive(Clone)]
pub struct IoVec {
    pub base: usize,
    pub len: usize,
}
