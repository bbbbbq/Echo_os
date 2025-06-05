use lazy_static::*;
use spin::Mutex;
use crate::Driver;
extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::DeviceType;
use crate::BlockDriver; // For the new get_block_device function

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


pub fn get_block_device(id: usize) -> Option<Arc<dyn BlockDriver>> {
    let device_arc = get_device(id)?;

    if device_arc.get_type() == DeviceType::Block {
        // Attempt to convert to the BlockDriver trait object using the object-safe method.
        device_arc.try_get_block_driver()
    } else {
        None // Not a block device
    }
}