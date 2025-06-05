use lazy_static::*;
use spin::Mutex;
use super::define::Driver;
extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::define::DeviceType;
use virtio_drivers::transport::DeviceType as VirtioDeviceType;


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