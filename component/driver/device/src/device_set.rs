use lazy_static::*;
use spin::Mutex;
use super::define::Driver;
extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
lazy_static!
{
    pub static ref DEVICE_SET:Mutex<Vec<Arc<dyn Driver>>> = Mutex::new(Vec::new());
}




