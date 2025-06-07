use mem::memset::MemSet;
extern crate alloc;
use alloc::sync::Arc;
use spin::Mutex;
use filesystem::file::File;



pub struct Process {
    pub mem: Arc<Mutex<MemSet>>,

}
