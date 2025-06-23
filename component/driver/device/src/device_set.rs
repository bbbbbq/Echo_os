//! 设备集合管理模块
//!
//! 提供设备注册、查找、类型转换等功能。

use crate::Driver;
use lazy_static::*;
use spin::Mutex;
extern crate alloc;
use crate::BlockDriver;
use crate::DeviceType;
use alloc::sync::Arc;
use alloc::vec::Vec; // For the new get_block_device function

/// 全局设备集合。
lazy_static! {
    pub static ref DEVICE_SET: Mutex<Vec<Arc<dyn Driver>>> = Mutex::new(Vec::new());
}

/// 注册一个设备到全局集合。
///
/// # 参数
/// * `device` - 设备对象。
pub fn push_device(device: Arc<dyn Driver>) {
    DEVICE_SET.lock().push(device);
}

/// 根据ID查找设备。
///
/// # 参数
/// * `id` - 设备ID。
/// # 返回
/// 找到则返回Some(设备对象)，否则返回None。
pub fn get_device(id: usize) -> Option<Arc<dyn Driver>> {
    let devices = DEVICE_SET.lock();
    for device in devices.iter() {
        if device.get_id() == id {
            return Some(Arc::clone(device));
        }
    }
    None
}

/// 根据ID查找块设备。
///
/// # 参数
/// * `id` - 设备ID。
/// # 返回
/// 找到则返回Some(块设备对象)，否则返回None。
pub fn get_block_device(id: usize) -> Option<Arc<dyn BlockDriver>> {
    let device_arc = get_device(id)?;

    if device_arc.get_type() == DeviceType::Block {
        // Attempt to convert to the BlockDriver trait object using the object-safe method.
        device_arc.try_get_block_driver()
    } else {
        None // Not a block device
    }
}
