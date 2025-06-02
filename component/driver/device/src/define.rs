extern crate alloc;
use alloc::sync::Arc;


pub enum DeviceType {
    Block,
    Network,
    Console,
    Unknown,
}


pub trait Driver: Send + Sync {
    fn get_id(&self) -> usize;
    fn get_type(&self) -> DeviceType;
}

pub trait BlockDriver: Driver {
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str>;
}

