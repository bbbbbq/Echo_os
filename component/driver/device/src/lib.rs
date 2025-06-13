#![no_std]

extern crate alloc;
extern crate log;

pub mod device_set;
pub use device_set::{DEVICE_SET, get_block_device, get_device, push_device};
pub use driver_api::{BlockDriver, DeviceType, Driver};

use alloc::boxed::Box;
use flat_device_tree::{Fdt, node::FdtNode};
use log::info;
// virtio_drivers::transport::DeviceType will be used via its full path
use core::ptr::NonNull;
use log::warn;
use virtio_drivers::transport::Transport;
use virtio_drivers::transport::mmio::MmioTransport;
use virtio_drivers::transport::mmio::VirtIOHeader;

pub fn init_dt(dtb: usize) {
    info!("device tree @ {:#x}", dtb);
    // Safe because the pointer is a valid pointer to unaliased memory.
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    walk_dt(fdt);
}

pub fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                virtio_probe(node);
            }
        }
    }
}

pub fn virtio_probe(node: FdtNode) {
    if let Some(reg) = node.reg().next() {
        let paddr = reg.starting_address as usize;
        let _size = reg.size.unwrap();
        let vaddr = paddr;
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        match unsafe { MmioTransport::new(header) } {
            Err(e) => warn!("Error creating VirtIO MMIO transport: {}", e),
            Ok(transport) => {
                virtio_device(transport);
            }
        }
    }
}

unsafe extern "C" {
    fn block_device(transport: *mut u8);
}

pub fn virtio_device(transport: impl Transport + Send + Sync + 'static) {
    match transport.device_type() {
        virtio_drivers::transport::DeviceType::Block => {
            let transport_box = Box::new(transport);
            let transport_ptr = Box::into_raw(transport_box) as *mut u8;
            unsafe { block_device(transport_ptr) };
        }
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}

pub fn get_mmio_start_end() -> (usize, usize) {
    let start = 0x10000000;
    let end = 0x1000f000;
    (start, end)
}
