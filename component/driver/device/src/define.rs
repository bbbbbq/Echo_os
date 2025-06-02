extern crate alloc;
use alloc::sync::Arc;
use core::any::Any;
use downcast_rs::{impl_downcast, DowncastSync};
#[derive(PartialEq)]
pub enum DeviceType {
    Block,
    Network,
    Console,
    Unknown,
}


pub trait Driver: Send + Sync + Any {
    fn get_id(&self) -> usize;
    fn get_type(&self) -> DeviceType;
    fn as_any(&self) -> &dyn Any;
}

pub trait BlockDriver: Driver {
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str>;
}
