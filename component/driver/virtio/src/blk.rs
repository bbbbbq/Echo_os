extern crate alloc;

use crate::halimpl::HalImpl;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::any::Any;
use driver_api::{BlockDriver, DeviceType, Driver};
use spin::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::Transport;
use virtio_drivers::transport::mmio::MmioTransport;
#[macro_use]
use uint_allocator::create_uint_allocator;
 // Renamed in Cargo.toml for virtio crate
use device_set::push_device; // Renamed in Cargo.toml for virtio crate
use log::{info, trace};

pub struct VirtioBlkDriver<T>
where
    T: Transport + Send + Sync,
{
    pub inner: Mutex<VirtIOBlk<HalImpl, T>>,
    pub id: usize,
}

impl<T> Driver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
{
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn try_get_block_driver(self: Arc<Self>) -> Option<Arc<dyn BlockDriver>>
    where
        Self: Sized + 'static, // 'static is often needed for Arc<dyn Trait>
    {
        Some(self) // VirtioBlkDriver is a BlockDriver.
    }
}

impl<T> BlockDriver for VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
{
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str> {
        match self.inner.lock().read_blocks(block_id as usize, buf) {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to read from block device"),
        }
    }

    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str> {
        match self.inner.lock().write_blocks(block_id as usize, buf) {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to write to block device"),
        }
    }

    fn capacity(&self) -> u64 {
        self.inner.lock().capacity()
    }
}

create_uint_allocator!(VIRTIO_DRIVER_ID, 0, 1024);

impl<T> VirtioBlkDriver<T>
where
    T: Transport + Send + Sync + 'static,
{
    pub fn new(inner: VirtIOBlk<HalImpl, T>) -> Self {
        let id = VIRTIO_DRIVER_ID
            .lock()
            .alloc()
            .expect("Failed to allocate driver ID");
        trace!("Creating new VirtioBlkDriver with ID: {}", id);
        Self {
            inner: Mutex::new(inner),
            id,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn block_device(transport_ptr: *mut u8) {
    let transport_box = unsafe { Box::from_raw(transport_ptr as *mut MmioTransport) };
    let transport = *transport_box;

    let blk =
        VirtIOBlk::<HalImpl, MmioTransport>::new(transport).expect("failed to create blk driver");
    // Create the concrete VirtioBlkDriver instance wrapped in Arc
    let concrete_virtio_blk_driver = Arc::new(VirtioBlkDriver::new(blk));

    // Explicitly upcast to the trait object Arc<dyn Driver>
    // This clarifies the type conversion to the `Driver` trait object.
    let driver_object: Arc<dyn Driver> = concrete_virtio_blk_driver;

    // Push the trait object to the device set
    push_device(driver_object);
    info!("Registered virtio block device");
}
