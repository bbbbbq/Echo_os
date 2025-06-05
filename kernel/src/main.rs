#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use core::ptr::NonNull;
use device::init_dt;
use filesystem::init_fs;
use flat_device_tree;
use flat_device_tree::{node::FdtNode, standard_nodes::Compatible, Fdt};
use heap;
use device::device_set::{get_device, get_block_device}; // Changed to get_block_device
use virtio::blk::VirtioBlkDriver;
use device::{Driver, BlockDriver}; // define module removed
use device::DeviceType as EchoDeviceType; // define module removed
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


    let blk_dev = get_block_device(0)
        .expect("Failed to get device 0 as a block driver, or it's not a block device.");
    
    let mut buf = [0u8; 512];
    info!("Attempting to read from block device 0");
    blk_dev.read(0, &mut buf)
        .expect("Failed to read from block device 0.");


    info!("Successfully read from block device. Buffer starts with: {:x?}", &buf[0..8]);
    info!("kernel_end");
    loop {}
}