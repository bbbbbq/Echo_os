extern crate alloc;

use device::define::{BlockDriver, Driver, DeviceType};
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::Transport;
use spin::Mutex;
use alloc::sync::Arc;
use crate::halimpl::HalImpl;
use UintAllocator::create_uint_allocator;


pub struct VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    pub inner: Mutex<VirtIOBlk<HalImpl, T>>,
    pub id: usize
}

impl<T> Driver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    fn get_id(&self) -> usize
    {
        self.id
    }

    fn get_type(&self) -> DeviceType
    {
        DeviceType::Block
    }
}

impl<T> BlockDriver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str> {
        match self.inner.lock().read_blocks(block_id as usize, buf) {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to read from block device")
        }
    }

    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str> {
        match self.inner.lock().write_blocks(block_id as usize, buf) {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to write to block device")
        }
    }
}


create_uint_allocator!(VIRTIO_DRIVER_ID, 0, 1024);


impl<T> VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    pub fn new(inner: VirtIOBlk<HalImpl, T>) -> Self
    {
        let id = VIRTIO_DRIVER_ID.lock().alloc().expect("Failed to allocate driver ID");
        Self {
            inner: Mutex::new(inner),
            id
        }
    }
}