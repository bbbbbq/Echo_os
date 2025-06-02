extern crate alloc;
use alloc::sync::Arc;
use core::any::Any;


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
    
    // Add a method to safely access block driver functionality if available
    fn as_block_driver(&self) -> Option<&dyn BlockDriver> {
        None // Default implementation returns None
    }
}

pub trait BlockDriver: Driver {
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str>;
}

