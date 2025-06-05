#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use core::ptr::NonNull;
use device::define::BlockDriver;
use device::init_dt;
use filesystem::init_fs;
use flat_device_tree;
use flat_device_tree::{node::FdtNode, standard_nodes::Compatible, Fdt};
use heap;
use device::device_set::get_device;
use virtio::blk::VirtioBlkDriver;
use device::define::Driver;
use device::define::DeviceType as EchoDeviceType; // Alias for clarity
use log::{debug, error, info, warn};
use virtio::halimpl::HalImpl;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};
extern crate alloc;
use alloc::vec;
use boot;
use device::device_set;
use frame;
use alloc::sync::Arc;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[panic] Panicked at {}:{} \n\t{}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        error!("[panic] Panicked: {}", info.message());
    }
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hartid: usize, dtb: usize) -> ! {
    console::init();
    println!("hart_id : {:x} dtb: {:x}", hartid, dtb);
    heap::init();

    init_dt(dtb);

    let device_option = get_device(0);
    if let Some(device_arc) = device_option {
        if device_arc.get_type() == EchoDeviceType::Block {
            match device_arc.downcast_arc::<VirtioBlkDriver<MmioTransport>>() {
                Ok(blk_driver_concrete) => {
                    let dev: Arc<dyn BlockDriver> = blk_driver_concrete;
                    let mut buf = [0u8; 512];
                    info!("Attempting to read from block device 0");
                    dev.read(0, &mut buf);
                    info!("Successfully read from block device. Buffer starts with: {:x?}", &buf[0..8]);
                }
                Err(_) => {
                    error!("Failed to downcast device 0 to VirtioBlkDriver");
                }
            }
        } else {
            error!("Device 0 is not a block device. Type: {:?}", device_arc.get_type());
        }
    } else {
        error!("Failed to get device 0");
    }
    info!("kernel_end");
    loop {}
}