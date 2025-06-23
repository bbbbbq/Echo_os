#![no_std]

//! 设备管理模块
//!
//! 提供设备树解析、virtio设备探测与注册等功能。

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

/// 解析设备树并初始化设备。
///
/// # 参数
/// * `dtb` - 设备树物理地址。
pub fn init_dt(dtb: usize) {
    info!("device tree @ {:#x}", dtb);
    // Safe because the pointer is a valid pointer to unaliased memory.
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    walk_dt(fdt);
}

/// 遍历设备树节点，探测virtio设备。
///
/// # 参数
/// * `fdt` - 设备树对象。
pub fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                virtio_probe(node);
            }
        }
    }
}

/// 探测并注册virtio设备。
///
/// # 参数
/// * `node` - 设备树节点。
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
    /// 注册块设备的外部C函数接口。
    fn block_device(transport: *mut u8);
}

/// 注册virtio设备到设备集。
///
/// # 参数
/// * `transport` - virtio传输对象。
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

/// 获取MMIO设备的起止地址范围。
///
/// # 返回
/// (起始地址, 结束地址)
pub fn get_mmio_start_end() -> (usize, usize) {
    let start = 0x10000000;
    let end = 0x1000f000;
    (start, end)
}
