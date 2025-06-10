#![no_std]

extern crate alloc;

pub mod blk;
pub mod halimpl;

use core::ptr::NonNull;
use flat_device_tree::{Fdt, node::FdtNode};
use log::{info, warn};
use virtio_drivers::transport::DeviceType;
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
