use lazy_static::*;
use spin::Mutex;
use super::define::Driver;
extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::define::DeviceType;
use virtio_drivers::transport::DeviceType as VirtioDeviceType;
use crate::define::BlockDriver;
use downcast_rs::{impl_downcast, DowncastSync};


lazy_static!
{
    pub static ref DEVICE_SET:Mutex<Vec<Arc<dyn Driver>>> = Mutex::new(Vec::new());
}

pub fn push_device(device: Arc<dyn Driver>) 
{
    DEVICE_SET.lock().push(device);
}


pub fn get_device(id: usize) -> Option<Arc<dyn Driver>> {
    let devices = DEVICE_SET.lock();
    for device in devices.iter() {
        if device.get_id() == id {
            return Some(Arc::clone(device));
        }
    }
    None
}

pub fn get_blk_device(id: usize) -> Option<Arc<dyn BlockDriver>> {
    let device = get_device(id)?;
    let device_type = device.get_type();
    if device_type == DeviceType::Block {
        unsafe {
            let raw_ptr = Arc::into_raw(device);
            let block_driver_ptr = core::mem::transmute::<*const dyn Driver, *const dyn BlockDriver>(raw_ptr);
            return Some(Arc::from_raw(block_driver_ptr));
        }
    }
    None
}