extern crate alloc;

use device::define::{BlockDriver, Driver, DeviceType};
use alloc::boxed::Box;
use core::any::Any;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use virtio_drivers::transport::Transport;
use spin::Mutex;
use alloc::sync::Arc;
use crate::halimpl::HalImpl;
use UintAllocator::create_uint_allocator;
use device::device_set::DEVICE_SET;
use device::device_set::push_device;
use log::{info,trace};

pub struct VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    pub inner: Mutex<VirtIOBlk<HalImpl, T>>,
    pub id: usize
}

impl<T> Driver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
{
    fn get_id(&self) -> usize
    {
        self.id
    }

    fn get_type(&self) -> DeviceType
    {
        DeviceType::Block
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T> BlockDriver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
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

    fn capacity(&self)->u64
    {
        self.inner.lock().capacity()
    }
}

create_uint_allocator!(VIRTIO_DRIVER_ID, 0, 1024);

impl<T> VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
{
    pub fn new(inner: VirtIOBlk<HalImpl, T>) -> Self
    {
        let id = VIRTIO_DRIVER_ID.lock().alloc().expect("Failed to allocate driver ID");
        trace!("Creating new VirtioBlkDriver with ID: {}", id);
        Self {
            inner: Mutex::new(inner),
            id
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn block_device(transport_ptr: *mut u8)
{
    let transport_box = unsafe { Box::from_raw(transport_ptr as *mut MmioTransport) };
    let transport = *transport_box;
    
    let blk = VirtIOBlk::<HalImpl, MmioTransport>::new(transport).expect("failed to create blk driver");
    let blk_device = Arc::new(VirtioBlkDriver::new(blk));
    push_device(blk_device);
    info!("Registered virtio block device");
}