pub mod list;
pub mod flages;

#[derive(Debug, Clone, Copy)]
pub struct SigProcMask {
    pub mask: usize,
}



#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SigMaskHow {
    Block,
    Unblock,
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