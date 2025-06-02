#![no_std]

extern crate alloc;

pub mod halimpl;
pub mod blk;

use flat_device_tree::{node::FdtNode, Fdt};
use log::{info, warn};
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};
use virtio_drivers::transport::DeviceType;
use core::ptr::NonNull;