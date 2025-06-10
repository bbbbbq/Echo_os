#![no_std]

use core::any::Any;
use downcast_rs::{impl_downcast, DowncastSync};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Network,
    Gpu,
    Input,
    Rtc,
    Serial,
    Timer,
    Misc,
}

extern crate alloc;

pub trait Driver: DowncastSync + Send + Sync {
    fn get_id(&self) -> usize;
    fn get_type(&self) -> DeviceType;
    fn as_any(&self) -> &dyn Any;

    /// Attempts to convert this driver object into an Arc<dyn BlockDriver>.
    /// Implementers should override this. Block drivers return Some(self), others None.
    fn try_get_block_driver(
        self: alloc::sync::Arc<Self>,
    ) -> Option<alloc::sync::Arc<dyn BlockDriver>>;
}
impl_downcast!(sync Driver);

pub trait BlockDriver: DowncastSync + Driver {
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str>;
    fn capacity(&self) -> u64;
}
